use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(super) fn run(project_slug: &str, name: &str, abs_path: &str, part_type: &str) -> i32 {
    if !abs_path.starts_with('/') {
        let msg = format!("error: path must be absolute: {abs_path}");
        if let Err(e) = ewrite_or_exit(&msg) {
            return into_exit_code(e);
        }
        return 1;
    }

    // RPC-first; direct fallback only when the daemon is unreachable. The direct
    // path additionally runs validate_project_workdir (a local-filesystem guard
    // the shared daemon cannot perform). SOURCE: https://github.com/facebook/rocksdb/issues/1780
    match super::rpc_client::register_part(project_slug, name, abs_path, part_type) {
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
            if let Err(io_err) = ewrite_or_exit("error: project missing id") {
                return into_exit_code(io_err);
            }
            return 1;
        };

        match kavach_surreal::part_upsert(&db, &project_id, name, abs_path, part_type, None, None)
            .await
        {
            Ok(id) => {
                let msg = format!("registered part: {name} ({part_type}) id={id:?} at {abs_path}");
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
