use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(super) fn run_projects() -> i32 {
    // RPC-first: route through the single-writer daemon. Fall back to a direct
    // (resilient) SurrealDB open ONLY when the daemon is unreachable — opening a
    // second RocksDB handle while the daemon holds the fcntl lock would race it.
    // SOURCE: https://github.com/facebook/rocksdb/issues/1780
    match super::rpc_client::list_projects() {
        Ok(res) => return print_projects(&res),
        Err(e) if super::rpc_client::should_fallback_to_direct(&e) => {}
        Err(e) => {
            let msg = format!("error: {e}");
            return ewrite_or_exit(&msg).map_or_else(into_exit_code, |()| 1);
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

        match kavach_surreal::projects_list_all(&db).await {
            Ok(projects) => {
                if projects.is_empty() {
                    if let Err(io_err) = print_or_exit("no projects registered") {
                        return into_exit_code(io_err);
                    }
                    return 0;
                }
                for p in &projects {
                    let path = p.workdir.as_deref().unwrap_or("(no path)");
                    let stack = p.stack.as_deref().unwrap_or("");
                    let line = format!("{} — {} [{stack}]", p.slug, path);
                    if let Err(io_err) = print_or_exit(&line) {
                        return into_exit_code(io_err);
                    }
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

/// Render the RPC `list_projects` result, mirroring the direct-path output so
/// the daemon and fallback produce byte-identical lines.
fn print_projects(res: &kavach_rpc::methods::db::ListProjectsResult) -> i32 {
    if res.projects.is_empty() {
        return print_or_exit("no projects registered").map_or_else(into_exit_code, |()| 0);
    }
    for p in &res.projects {
        let path = p.workdir.as_deref().unwrap_or("(no path)");
        let stack = p.stack.as_deref().unwrap_or("");
        let line = format!("{} — {} [{stack}]", p.slug, path);
        if let Err(io_err) = print_or_exit(&line) {
            return into_exit_code(io_err);
        }
    }
    0
}

/// Render the RPC `list_parts` result, mirroring the direct-path output.
fn print_parts(res: &kavach_rpc::methods::db::ListPartsResult, project_slug: &str) -> i32 {
    if res.parts.is_empty() {
        let msg = format!("no parts for {project_slug}");
        return print_or_exit(&msg).map_or_else(into_exit_code, |()| 0);
    }
    for p in &res.parts {
        let stack = p.stack.as_deref().unwrap_or("");
        let line = format!(
            "{} ({}) — {} [{stack}]",
            p.part_name, p.part_type, p.part_path
        );
        if let Err(io_err) = print_or_exit(&line) {
            return into_exit_code(io_err);
        }
    }
    0
}

pub(super) fn run_parts(project_slug: &str) -> i32 {
    // RPC-first, fall back to a resilient direct open only when the daemon is
    // unreachable. SOURCE: https://github.com/facebook/rocksdb/issues/1780
    match super::rpc_client::list_parts(project_slug) {
        Ok(res) => return print_parts(&res, project_slug),
        Err(e) if super::rpc_client::should_fallback_to_direct(&e) => {}
        Err(e) => {
            let msg = format!("error: {e}");
            return ewrite_or_exit(&msg).map_or_else(into_exit_code, |()| 1);
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

        let Some(project_id) = project.id else {
            if let Err(io_err) = ewrite_or_exit("error: project missing id") {
                return into_exit_code(io_err);
            }
            return 1;
        };

        match kavach_surreal::parts_list_by_project(&db, &project_id).await {
            Ok(parts) => {
                if parts.is_empty() {
                    let msg = format!("no parts for {project_slug}");
                    if let Err(io_err) = print_or_exit(&msg) {
                        return into_exit_code(io_err);
                    }
                    return 0;
                }
                for p in &parts {
                    let stack = p.stack.as_deref().unwrap_or("");
                    let line = format!(
                        "{} ({}) — {} [{stack}]",
                        p.part_name, p.part_type, p.part_path
                    );
                    if let Err(io_err) = print_or_exit(&line) {
                        return into_exit_code(io_err);
                    }
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
