// `kavach db status-update` — set strict entry_status for a memory entry.
// SurrealDB-backed: routes through kavach_surreal::update_status with table name.
// SOURCE: https://docs.rs/strum/0.28 — MemoryStatus uses strum EnumString + EnumIter.
use kavach_types::MemoryStatus;
use std::str::FromStr as _;

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

#[expect(
    clippy::too_many_lines,
    reason = "RPC-first with fallback to direct DB requires nested match arms and error handling"
)]
pub(super) fn run(
    project_slug: &str,
    category: &str,
    key: &str,
    status: &str,
) -> i32 {
    if MemoryStatus::from_str(status).is_err() {
        let msg = format!(
            "error: invalid status '{status}'. Valid: {}",
            MemoryStatus::allowed_list()
        );
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }

    // RPC-first: try daemon, fall back to direct DB on unavailable
    match super::rpc_client::status_update(project_slug, category, key, status) {
        Ok(result) if result.success => {
            let msg = format!("status applied: [{category}] {key} -> {status} (via rpc)");
            if let Err(io_err) = print_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 0;
        }
        Ok(result) => {
            let msg = format!(
                "error: {}",
                result.error.unwrap_or_else(|| "unknown".to_owned())
            );
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
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
        if let Err(e) = kavach_surreal::apply_schema(&db).await {
            let msg = format!("error: schema apply: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        let project = match kavach_surreal::project_get_by_slug(&db, project_slug).await {
            Ok(Some(p)) => p,
            Ok(None) => {
                let msg = format!("error: project not found: {project_slug}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
            Err(e) => {
                let msg = format!("error: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        };
        if let Err(code) = super::validate_project_workdir(&project) {
            return code;
        }
        let Some(project_id) = project.id else {
            if let Err(io_err) = ewrite_or_exit("error: project has no id") {
                return into_exit_code(io_err);
            }
            return 1;
        };
        match kavach_surreal::update_status(&db, category, &project_id, key, status).await {
            Ok(n) if n > 0 => {
                let msg = format!("status: [{category}] {key} → {status}");
                if let Err(io_err) = print_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                refresh_memory_entry_graph(&db, category, key, project_slug).await;
                0
            }
            Ok(_) => {
                let msg = format!("error: entry not found: {category}/{key}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                1
            }
            Err(e) => {
                let msg = format!("error: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                1
            }
        }
    })
}

/// Direct-DB graph projection refresh. Best-effort: errors logged via `.ok()`
/// because the primary status-update already succeeded. The graph refresh is
/// a secondary signal; primary stream success is the user-facing contract.
async fn refresh_memory_entry_graph(
    db: &surrealdb::Surreal<surrealdb::engine::local::Db>,
    category: &str,
    entry_key: &str,
    project_slug: &str,
) {
    use kavach_surreal::graph::dynamic::{relate_dynamic, upsert_entity};
    let qualified_name =
        kavach_engine::memory_entry_qualified_name(category, entry_key, project_slug);
    let entry_id = match upsert_entity(db, category, &qualified_name).await {
        Ok(id) => id,
        Err(e) => {
            let msg = format!("graph: upsert {category}/{qualified_name} failed: {e}");
            ewrite_or_exit(&msg).ok();
            return;
        }
    };
    if !project_slug.is_empty()
        && let Ok(proj_id) = upsert_entity(db, "project", project_slug).await
        && let Err(e) = relate_dynamic(db, &entry_id, &proj_id, "in_scope", 1.0).await
    {
        let msg = format!("graph: in_scope edge failed: {e}");
        ewrite_or_exit(&msg).ok();
    }
}
