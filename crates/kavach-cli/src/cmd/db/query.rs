use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::RecordId;

use super::kanban::is_done_title;
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

const ALL_CATEGORIES: &[&str] = &["decision", "research", "roadmap", "pattern", "app_spec"];

/// Resolved per-row content depth for query output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Depth {
    /// Titles only — the breadth view (no `--depth` given, no override).
    None,
    /// At most N chars of content per row.
    Chars(usize),
    /// The whole content body (`--depth all` or `KAVACH_NO_TRUNCATE=1`).
    All,
}

/// Resolve the depth from the `--depth` flag and the `KAVACH_NO_TRUNCATE` env.
/// The env override wins (forces `All`); else `all`/empty→`All`, an integer→
/// `Chars(n)`, a non-integer→`None` (fail-safe: a typo never dumps huge bodies).
#[must_use]
pub(crate) fn resolve_depth(flag: Option<&str>, no_truncate_env: bool) -> Depth {
    if no_truncate_env {
        return Depth::All;
    }
    match flag {
        None => Depth::None,
        Some(s) if s.eq_ignore_ascii_case("all") => Depth::All,
        Some(s) => s.trim().parse::<usize>().map_or(Depth::None, Depth::Chars),
    }
}

/// Truncate `content` to the resolved depth on a UTF-8 char boundary, appending an
/// ellipsis marker only when the body was actually cut. `Depth::None` yields `None`
/// (no content line). Never indexes by byte, so a multi-byte boundary cannot panic.
#[must_use]
pub(crate) fn render_content(content: &str, depth: Depth) -> Option<String> {
    let max = match depth {
        Depth::None => return None,
        Depth::All => return Some(content.to_owned()),
        Depth::Chars(n) => n,
    };
    let mut out = String::with_capacity(max.min(content.len()));
    for ch in content.chars().take(max) {
        out.push(ch);
    }
    if content.chars().nth(max).is_some() {
        out.push_str(" …[truncated; --depth all for full]");
    }
    Some(out)
}

#[expect(
    clippy::too_many_lines,
    reason = "RPC-first with direct-DB fallback requires both print paths inline"
)]
pub(super) fn run(
    project_slug: &str,
    category: Option<&str>,
    include_done: bool,
    depth_flag: Option<&str>,
) -> i32 {
    let depth = resolve_depth(
        depth_flag,
        std::env::var("KAVACH_NO_TRUNCATE").as_deref() == Ok("1"),
    );
    // ALGO: linear iteration to format output
    // PROBLEM_CLASS: print
    // TIME: O(n) | SPACE: O(1)
    // YEAR: 2026 | SEARCHED: 2026-05
    // BENCHMARK: https://doc.rust-lang.org/std/iter/trait.Iterator.html
    match super::rpc_client::query(project_slug, category, include_done) {
        Ok(result) => {
            for entry in &result.entries {
                let line = format!(
                    "[{}] {} — {} (status: {}, access: {})",
                    entry.category, entry.key, entry.title, entry.status, entry.access_count
                );
                if let Err(io_err) = print_or_exit(&line) {
                    return into_exit_code(io_err);
                }
                if let Some(body) =
                    entry.content.as_deref().and_then(|c| render_content(c, depth))
                    && let Err(io_err) = print_or_exit(&format!("    {body}"))
                {
                    return into_exit_code(io_err);
                }
            }
            return 0;
        }
        Err(e) if super::rpc_client::should_fallback_to_direct(&e) => {}
        Err(e) => {
            // Daemon is UP and holds the RocksDB lock — a direct open here
            // would race it (LOCK: Resource temporarily unavailable).
            let msg = format!("rpc error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    }

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("error: tokio runtime: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };

    runtime.block_on(async {
        // Resilient open — closes the daemon-restart TOCTOU
        // (`rca.db-event-daemon-restart-race`): retry the lock-acquiring open
        // (bounded) instead of trusting the socket proxy; a genuine stale
        // lock still surfaces after the backoff exhausts.
        let db = match super::rpc_client::open_direct_resilient().await {
            Ok(d) => d,
            Err(e) => {
                let msg = format!("error: open SurrealDB: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };

        let project_id = match resolve_project_id(&db, project_slug).await {
            Ok(id) => id,
            Err(code) => return code,
        };

        let tables: Vec<&str> = category.map_or_else(|| ALL_CATEGORIES.to_vec(), |c| vec![c]);

        let mut entries = Vec::new();
        for table in &tables {
            match kavach_surreal::list_by_project(&db, table, &project_id).await {
                Ok(rows) => entries.extend(rows),
                Err(e) => {
                    let msg = format!("error reading {table}: {e}");
                    if let Err(io_err) = ewrite_or_exit(&msg) {
                        return into_exit_code(io_err);
                    }
                    return 1;
                }
            }
        }

        let filtered: Vec<_> = if include_done {
            entries
        } else {
            entries
                .into_iter()
                .filter(|e| e.category_str() != "roadmap" || !is_done_title(&e.title))
                .collect()
        };

        if filtered.is_empty() {
            let label = category.unwrap_or("all");
            let suffix = if include_done {
                ""
            } else {
                " (use --all to include DONE items)"
            };
            let msg = format!("no entries for {project_slug} ({label}){suffix}");
            if let Err(io_err) = print_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 0;
        }

        for entry in &filtered {
            let line = format!(
                "[{}] {} — {} (access: {})",
                entry.category_str(),
                entry.entry_key,
                entry.title,
                entry.access_count.unwrap_or(0)
            );
            if let Err(io_err) = print_or_exit(&line) {
                return into_exit_code(io_err);
            }
            if let Some(body) = render_content(&entry.content, depth)
                && let Err(io_err) = print_or_exit(&format!("    {body}"))
            {
                return into_exit_code(io_err);
            }
        }
        0
    })
}

async fn resolve_project_id(db: &Surreal<Db>, slug: &str) -> Result<RecordId, i32> {
    match kavach_surreal::project_get_by_slug(db, slug).await {
        Ok(Some(p)) => {
            if let Some(id) = p.id {
                Ok(id)
            } else {
                if let Err(io_err) = ewrite_or_exit("error: project missing id") {
                    return Err(into_exit_code(io_err));
                }
                Err(1)
            }
        }
        Ok(None) => {
            let msg = format!("error: project not found: {slug}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return Err(into_exit_code(io_err));
            }
            Err(1)
        }
        Err(e) => {
            let msg = format!("error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return Err(into_exit_code(io_err));
            }
            Err(1)
        }
    }
}

#[cfg(test)]
#[path = "query_depth_test.rs"]
mod tests;
