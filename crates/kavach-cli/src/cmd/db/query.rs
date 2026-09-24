use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
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
/// `Chars(n)`, a non-integer→`None` (fail-safe: a typo never dumps huge bodies).\
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
pub(crate) struct QueryEntryView<'a> {
    pub category: &'a str,
    pub key: &'a str,
    pub title: &'a str,
    pub status: &'a str,
    pub access_count: i64,
    pub content: Option<&'a str>,
}

pub(crate) fn format_query_entry(
    view: &QueryEntryView<'_>,
    depth: Depth,
    field: Option<&str>,
    compact: bool,
) -> Result<Vec<String>, String> {
    if let Some(f) = field {
        let val = match f.trim().to_ascii_lowercase().as_str() {
            "key" | "entry_key" => view.key.to_owned(),
            "title" => view.title.to_owned(),
            "status" | "entry_status" => view.status.to_owned(),
            "category" => view.category.to_owned(),
            "access" | "access_count" => view.access_count.to_string(),
            "content" => {
                let raw = view.content.unwrap_or_default();
                if compact {
                    kavach_toon::compact::compress(raw, kavach_toon::compact::Level::Lite)
                } else {
                    raw.to_owned()
                }
            }
            unknown => return Err(format!("unknown field '{unknown}'. Supported: key, title, status, category, access, content")),
        };
        return Ok(vec![val]);
    }

    let mut lines = Vec::with_capacity(2);
    lines.push(format!(
        "[{}] {} — {} (status: {}, access: {})",
        view.category, view.key, view.title, view.status, view.access_count
    ));
    if let Some(body) = view.content.and_then(|c| render_content(c, depth)) {
        let final_body = if compact {
            kavach_toon::compact::compress(&body, kavach_toon::compact::Level::Lite)
        } else {
            body
        };
        lines.push(format!("    {final_body}"));
    }
    Ok(lines)
}

#[expect(
    clippy::too_many_lines,
    reason = "RPC-first with Turso direct-DB fallback requires both print paths inline"
)]
pub(super) fn run(
    project_slug: &str,
    category: Option<&str>,
    include_done: bool,
    depth_flag: Option<&str>,
    field: Option<&str>,
    compact: bool,
) -> i32 {
    use kavach_db::turso::entries::EntryStore;
    use super::kanban::is_done_title;
    let depth = resolve_depth(
        depth_flag,
        std::env::var("KAVACH_NO_TRUNCATE").as_deref() == Ok("1"),
    );
    match super::rpc_client::query(project_slug, category, include_done) {
        Ok(result) => {
            for entry in &result.entries {
                let view = QueryEntryView {
                    category: &entry.category,
                    key: &entry.key,
                    title: &entry.title,
                    status: &entry.status,
                    access_count: entry.access_count,
                    content: entry.content.as_deref(),
                };
                let lines = match format_query_entry(&view, depth, field, compact) {
                    Ok(l) => l,
                    Err(err_msg) => {
                        if let Err(io_err) = ewrite_or_exit(&format!("error: {err_msg}")) {
                            return into_exit_code(io_err);
                        }
                        return 1;
                    }
                };
                for line in lines {
                    if let Err(io_err) = print_or_exit(&line) {
                        return into_exit_code(io_err);
                    }
                }
            }
            return 0;
        }
        Err(e) if super::rpc_client::should_fallback_to_direct(&e) => {}
        Err(e) => {
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
        // Phase 6: direct open via TursoStore (libSQL file — no lock contention).
        let store = match super::rpc_client::open_turso_resilient().await {
            Ok(s) => s,
            Err(e) => {
                let msg = format!("error: open kavach.db: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };
        let entries = if let Some(cat) = category {
            match store.list_by_category(project_slug, cat).await {
                Ok(rows) => rows,
                Err(e) => {
                    let msg = format!("error reading {cat}: {e}");
                    if let Err(io_err) = ewrite_or_exit(&msg) {
                        return into_exit_code(io_err);
                    }
                    return 1;
                }
            }
        } else {
            match store.list_by_project(project_slug).await {
                Ok(rows) => rows,
                Err(e) => {
                    let msg = format!("error reading project {project_slug}: {e}");
                    if let Err(io_err) = ewrite_or_exit(&msg) {
                        return into_exit_code(io_err);
                    }
                    return 1;
                }
            }
        };
        let filtered: Vec<_> = if include_done {
            entries
        } else {
            entries
                .into_iter()
                .filter(|e| {
                    e.category.as_deref().unwrap_or("") != "roadmap"
                        || !is_done_title(&e.title)
                })
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
            let cat_str = entry.category.as_deref().unwrap_or("unknown");
            // K-1: single read path (effective_status) so query agrees with
            // context/kanban on legacy/empty-status rows.
            let status_str = entry.effective_status();
            let view = QueryEntryView {
                category: cat_str,
                key: &entry.entry_key,
                title: &entry.title,
                status: status_str,
                access_count: entry.access_count.unwrap_or(0),
                content: Some(&entry.content),
            };
            let lines = match format_query_entry(&view, depth, field, compact) {
                Ok(l) => l,
                Err(err_msg) => {
                    if let Err(io_err) = ewrite_or_exit(&format!("error: {err_msg}")) {
                        return into_exit_code(io_err);
                    }
                    return 1;
                }
            };
            for line in lines {
                if let Err(io_err) = print_or_exit(&line) {
                    return into_exit_code(io_err);
                }
            }
        }
        0
    })
}
#[cfg(test)]
#[path = "query_depth_test.rs"]
mod tests;
