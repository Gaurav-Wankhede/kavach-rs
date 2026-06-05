use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

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

        match kavach_surreal::project_register(&db, slug, slug, abs_path, stack).await {
            Ok(id) => {
                let msg = format!("registered project: {slug} (id={id:?}) at {abs_path}");
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
