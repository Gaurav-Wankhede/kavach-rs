use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb_types::RecordId;

use super::kanban::is_done_title;
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

const ALL_CATEGORIES: &[&str] = &["decision", "research", "roadmap", "pattern", "app_spec"];

pub(super) fn run(project_slug: &str, category: Option<&str>, include_done: bool) -> i32 {
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
