// SOURCE: https://docs.rs/kavach-surreal — FilterBuilder API
use kavach_db::turso::entries::EntryStore;

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

const ALL_CATEGORIES: &[&str] = &["decision", "research", "roadmap", "pattern", "app_spec"];

pub(super) fn run(
    project_slug: &str,
    category: Option<&str>,
    status: Option<&str>,
    since: Option<&str>,
    contains: Option<&str>,
    limit: usize,
) -> i32 {
    // RPC-first; direct fallback only when the daemon is unreachable.
    // SOURCE: https://github.com/facebook/rocksdb/issues/1780
    match super::rpc_client::search(project_slug, category, status, since, contains, limit) {
        Ok(res) => {
            if res.entries.is_empty() {
                return print_or_exit("no matching entries").map_or_else(into_exit_code, |()| 0);
            }
            for hit in &res.entries {
                let line = format!(
                    "[{}] {} — {} (status: {})",
                    hit.category, hit.key, hit.title, hit.status
                );
                if let Err(io_err) = print_or_exit(&line) {
                    return into_exit_code(io_err);
                }
            }
            return 0;
        }
        Err(e) if super::rpc_client::should_fallback_to_direct(&e) => {}
        Err(e) => {
            return ewrite_or_exit(&format!("error: {e}")).map_or_else(into_exit_code, |()| 1);
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

        let tables: Vec<&str> = category.map_or_else(|| ALL_CATEGORIES.to_vec(), |c| vec![c]);

        let mut all_entries = Vec::new();
        let limit_i64 = i64::try_from(limit).map_or(i64::MAX, |l| l);
        for table in &tables {
            match store
                .list_with_filter(project_slug, Some(table), status, contains, limit_i64)
                .await
            {
                Ok(rows) => all_entries.extend(rows),
                Err(e) => {
                    let msg = format!("error reading {table}: {e}");
                    if let Err(io_err) = ewrite_or_exit(&msg) {
                        return into_exit_code(io_err);
                    }
                    return 1;
                }
            }
        }

        // Sort by updated_at descending (most recent first)
        all_entries.sort_by(|a, b| {
            let a_ts = a.updated_at.unwrap_or_else(chrono::Utc::now);
            let b_ts = b.updated_at.unwrap_or_else(chrono::Utc::now);
            b_ts.cmp(&a_ts)
        });
        all_entries.truncate(limit);

        if all_entries.is_empty() {
            if let Err(io_err) = print_or_exit("no matching entries") {
                return into_exit_code(io_err);
            }
            return 0;
        }

        for entry in &all_entries {
            let cat_str = entry.category.as_deref().unwrap_or("unknown");
            let status_str = entry.entry_status.as_deref().unwrap_or("todo");
            let line = format!(
                "[{}] {} — {} (status: {})",
                cat_str, entry.entry_key, entry.title, status_str
            );
            if let Err(io_err) = print_or_exit(&line) {
                return into_exit_code(io_err);
            }
        }
        0
    })
}
