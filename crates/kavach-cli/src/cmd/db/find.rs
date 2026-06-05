use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(super) fn run_project(abs_path: &str) -> i32 {
    if !abs_path.starts_with('/') {
        let msg = format!("error: path must be absolute: {abs_path}");
        if let Err(e) = ewrite_or_exit(&msg) {
            return into_exit_code(e);
        }
        return 1;
    }

    // RPC-first; direct fallback only when the daemon is unreachable.
    // SOURCE: https://github.com/facebook/rocksdb/issues/1780
    match super::rpc_client::find_project(abs_path) {
        Ok(res) => {
            return match res.label {
                Some(slug) => {
                    let path = res.detail.as_deref().unwrap_or("?");
                    print_or_exit(&format!("{slug} (path={path})"))
                        .map_or_else(into_exit_code, |()| 0)
                }
                None => ewrite_or_exit(&format!("no project matches: {abs_path}"))
                    .map_or_else(into_exit_code, |()| 1),
            };
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

        match kavach_surreal::project_find_by_path(&db, abs_path).await {
            Ok(Some(p)) => {
                let msg = format!("{} (path={})", p.slug, p.workdir.as_deref().unwrap_or("?"));
                if let Err(io_err) = print_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                0
            }
            Ok(None) => {
                let msg = format!("no project matches: {abs_path}");
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

pub(super) fn run_part(abs_path: &str) -> i32 {
    if !abs_path.starts_with('/') {
        let msg = format!("error: path must be absolute: {abs_path}");
        if let Err(e) = ewrite_or_exit(&msg) {
            return into_exit_code(e);
        }
        return 1;
    }

    // RPC-first; direct fallback only when the daemon is unreachable.
    // SOURCE: https://github.com/facebook/rocksdb/issues/1780
    match super::rpc_client::find_part(abs_path) {
        Ok(res) => {
            return match res.label {
                Some(name) => {
                    let path = res.detail.as_deref().unwrap_or("?");
                    print_or_exit(&format!("{name} (path={path})"))
                        .map_or_else(into_exit_code, |()| 0)
                }
                None => ewrite_or_exit(&format!("no part matches: {abs_path}"))
                    .map_or_else(into_exit_code, |()| 1),
            };
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

        match kavach_surreal::part_find_by_path(&db, abs_path).await {
            Ok(Some(p)) => {
                let msg = format!("{} ({}, path={})", p.part_name, p.part_type, p.part_path);
                if let Err(io_err) = print_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
                0
            }
            Ok(None) => {
                let msg = format!("no part matches: {abs_path}");
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
