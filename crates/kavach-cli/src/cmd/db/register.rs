use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use kavach_db::turso::entries::EntryStore;

pub(super) fn run(slug: &str, abs_path: &str, stack: Option<&str>) -> i32 {
    if !abs_path.starts_with('/') {
        let msg = format!("error: path must be absolute: {abs_path}");
        if let Err(e) = ewrite_or_exit(&msg) {
            return into_exit_code(e);
        }
        return 1;
    }

    // RPC-first; direct fallback only when the daemon is unreachable.
    // SOURCE: https://github.com/facebook/rocksdb/issues/1780
    match super::rpc_client::register(slug, abs_path, stack) {
        Ok(res) => return print_or_exit(&res.message).map_or_else(into_exit_code, |()| 0),
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

        // Insert or update project in the projects table
        let tags_json = stack.map(|s| format!("[\"{s}\"]"));
        if let Err(e) = store.register_project(slug, abs_path, tags_json.as_deref()).await {
            let msg = format!("error: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        // K-6: read back the stored row so a silently-dropped `path` fails
        // loudly instead of reporting success while the column stays null.
        match kavach_db::project_get_by_slug(&store, slug).await {
            Ok(Some(p)) if p.workdir.as_deref() == Some(abs_path) => {}
            Ok(Some(p)) => {
                let msg = format!(
                    "error: register wrote slug={slug} but stored path={:?}, expected {abs_path}",
                    p.workdir
                );
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
            Ok(None) => {
                let msg = format!("error: register succeeded but {slug} not found on read-back");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
            Err(e) => {
                let msg = format!("error: register read-back failed: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                return 1;
            }
        }
        let msg = format!("registered project: {slug} at {abs_path}");
        if let Err(io_err) = print_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        0
    })
}
