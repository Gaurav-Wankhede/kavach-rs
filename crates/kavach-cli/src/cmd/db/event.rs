use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use kavach_db::key_str::project_key_str;
use kavach_db::turso::entries::EntryStore;

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
        let session_db = kavach_session::get_or_create_session();
        let project_slug = if session_db.work_dir.is_empty() {
            "default".to_owned()
        } else {
            // Try to find project by path in Turso
            match store.list_all().await {
                Ok(entries) => {
                    let mut found = None;
                    for e in entries {
                        let slug = project_key_str(&e.project);
                        if slug == session_db.work_dir {
                            found = Some(slug);
                            break;
                        }
                    }
                    found.unwrap_or_else(|| "default".to_owned())
                }
                Err(_) => "default".to_owned(),
            }
        };
        match store.append_event(&project_slug, event_type, payload.unwrap_or("")).await {
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
