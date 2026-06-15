use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

/// Build the session-snapshot JSON payload written as a `session_sync` event.
fn snapshot_payload(session: &kavach_session::SessionState) -> String {
    serde_json::json!({
        "session_id": session.session_id,
        "turn_count": session.turn_count,
        "research_done": session.research_done,
        "files_modified": session.files_modified,
        "tasks_created": session.tasks_created,
        "tasks_completed": session.tasks_completed,
        "context_phase": session.context_phase,
    })
    .to_string()
}

/// Sync current session state to `SurrealDB` as an event.
///
/// RPC-first (single-writer invariant — decision `rocksdb-lock-fix`): route the
/// snapshot through the kavach-rpc daemon so this short-lived hook child never
/// opens a competing `RocksDB` handle while the daemon holds the `fcntl` lock.
/// Opening a second handle is exactly what wedged the `SessionEnd` hook with
/// `LOCK: Resource temporarily unavailable` (rocksdb#3114 daemon-restart TOCTOU).
/// Only when the daemon is genuinely unreachable do we fall back to a direct,
/// resilience-bounded open — the same flow every other event writer uses
/// (`db/event.rs`).
pub(super) fn run() -> i32 {
    let session = kavach_session::get_or_create_session();
    let payload = snapshot_payload(&session);

    match super::rpc_client::event("session_sync", Some(&payload), &session.work_dir) {
        Ok(r) if r.success => {
            if let Err(io_err) = print_or_exit("synced session state to SurrealDB (via rpc)") {
                return into_exit_code(io_err);
            }
            return 0;
        }
        Ok(r) => {
            let msg = format!("error: {}", super::rpc_client::or_str(r.error, "unknown"));
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        // Daemon unreachable — no process holds the lock, a direct open is safe.
        Err(e) if super::rpc_client::should_fallback_to_direct(&e) => {}
        Err(e) => {
            // Daemon is UP and holds the RocksDB lock — a direct open here would
            // race it. Propagate instead of opening a second handle.
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
        // Resilient open: closes the daemon-restart TOCTOU. A mid-restart daemon
        // may hold the lock between the socket check and this open; retry the
        // lock-acquiring action itself (bounded) instead of trusting the socket
        // proxy. A genuine stale lock still surfaces after exhaustion.
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
        let project_id = resolve_project_id(&db, &session.project, &session.work_dir).await;
        match kavach_surreal::append_event(
            &db,
            "session_sync",
            "kavach-cli",
            project_id,
            Some(&payload),
        )
        .await
        {
            Ok(_) => {
                if let Err(io_err) = print_or_exit("synced session state to SurrealDB") {
                    return into_exit_code(io_err);
                }
                0
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

async fn resolve_project_id(
    db: &surrealdb::Surreal<surrealdb::engine::local::Db>,
    slug: &str,
    workdir: &str,
) -> Option<surrealdb_types::RecordId> {
    if !slug.is_empty()
        && let Ok(Some(p)) = kavach_surreal::project_get_by_slug(db, slug).await
    {
        return p.id;
    }
    if !workdir.is_empty()
        && let Ok(Some(p)) = kavach_surreal::project_find_by_path(db, workdir).await
    {
        return p.id;
    }
    None
}
