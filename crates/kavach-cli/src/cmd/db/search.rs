// ALGO: linear filter composition + DB query — O(n) where n = filter count
// SOURCE: https://docs.rs/kavach-surreal — FilterBuilder API
use kavach_surreal::{FilterBuilder, FilterExpr};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb_types::RecordId;

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

        let filter = build_filter(status, since, contains);
        let tables: Vec<&str> = category.map_or_else(|| ALL_CATEGORIES.to_vec(), |c| vec![c]);

        let mut all_entries = Vec::new();
        for table in &tables {
            match kavach_surreal::list_with_filter(
                &db,
                table,
                &project_id,
                filter.as_ref(),
                Some(limit),
            )
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

        all_entries.sort_by_key(|e| std::cmp::Reverse(e.updated_at));
        all_entries.truncate(limit);

        if all_entries.is_empty() {
            if let Err(io_err) = print_or_exit("no matching entries") {
                return into_exit_code(io_err);
            }
            return 0;
        }

        for entry in &all_entries {
            let status_str = if entry.entry_status_str().is_empty() {
                entry.status_str()
            } else {
                entry.entry_status_str()
            };
            let line = format!(
                "[{}] {} — {} (status: {})",
                entry.category_str(),
                entry.entry_key,
                entry.title,
                status_str
            );
            if let Err(io_err) = print_or_exit(&line) {
                return into_exit_code(io_err);
            }
        }
        0
    })
}

const ALLOWED_STATUSES: &[&str] = &["todo", "in_progress", "done", "verified"];

fn is_valid_duration(s: &str) -> bool {
    if s.is_empty() || s.len() > 16 {
        return false;
    }
    let last = match s.as_bytes().last() {
        Some(b) => *b,
        None => return false,
    };
    if !matches!(last, b'd' | b'h' | b'm' | b's' | b'w' | b'y') {
        return false;
    }
    let Some(digits) = s.get(..s.len().saturating_sub(1)) else {
        return false;
    };
    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
}

fn build_filter(
    status: Option<&str>,
    since: Option<&str>,
    contains: Option<&str>,
) -> Option<FilterExpr> {
    let mut builder = FilterBuilder::new();

    if let Some(s) = status
        && ALLOWED_STATUSES.contains(&s)
    {
        builder = builder.eq("entry_status", s);
    }
    if let Some(d) = since
        && is_valid_duration(d)
    {
        builder = builder.since("updated_at", d);
    }
    if let Some(c) = contains {
        builder = builder.contains("title", c);
    }

    builder.build()
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
