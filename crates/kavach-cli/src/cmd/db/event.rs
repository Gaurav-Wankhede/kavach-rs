use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(super) fn run(event_type: &str, payload: Option<&str>) -> i32 {
    // RPC-first: route through the daemon so the CLI never opens a competing
    // RocksDB handle (single-writer invariant — decision `rocksdb-lock-fix`).
    let session = kavach_session::get_or_create_session();
    match super::rpc_client::event(event_type, payload, &session.work_dir) {
        Ok(r) if r.success => {
            // Daemon sets id=Some(..) whenever success=true; the `else`
            // is unreachable in practice but kept total (no unwrap).
            let msg = r.id.map_or_else(
                || format!("event {event_type} logged (via rpc)"),
                |id| format!("event {event_type} logged (id={id}) (via rpc)"),
            );
            if let Err(io_err) = print_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 0;
        }
        Ok(r) => {
            let msg_text = super::rpc_client::or_str(r.error, "unknown");
            let msg = format!("error: {msg_text}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
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
        // Resilient open: closes the daemon-restart TOCTOU
        // (`rca.db-event-daemon-restart-race`). A mid-restart daemon may hold
        // the RocksDB lock between our socket check and this open; retry the
        // lock-acquiring action itself (bounded) instead of trusting the
        // socket proxy. A genuine stale lock still surfaces after exhaustion.
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
        let session_db = kavach_session::get_or_create_session();
        let project_id = if session_db.work_dir.is_empty() {
            None
        } else {
            match kavach_surreal::project_find_by_path(&db, &session_db.work_dir).await {
                Ok(Some(p)) => p.id,
                Ok(None) => None,
                Err(e) => {
                    eprintln!("event: project lookup failed (continuing without project): {e}");
                    None
                }
            }
        };
        match kavach_surreal::append_event(&db, event_type, "kavach-cli", project_id, payload).await
        {
            Ok(id) => {
                let msg = format!("event {event_type} logged (id={id:?})");
                if let Err(io_err) = print_or_exit(&msg) {
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
