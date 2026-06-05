use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use kavach_surreal::{Project, ProjectNode};

/// `kavach db set-parent --child <slug> [--parent <slug>]`
pub(super) fn set_parent(child: &str, parent: Option<&str>) -> i32 {
    // RPC-first; direct fallback only when the daemon is unreachable.
    // SOURCE: https://github.com/facebook/rocksdb/issues/1780
    match super::rpc_client::set_parent(child, parent) {
        Ok(res) => return print_or_exit(&res.message).map_or_else(into_exit_code, |()| 0),
        Err(e) if super::rpc_client::should_fallback_to_direct(&e) => {}
        Err(e) => return report_err(&e),
    }

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => return report_err(&format!("tokio runtime: {e}")),
    };

    runtime.block_on(async {
        let db = match super::rpc_client::open_direct_resilient().await {
            Ok(d) => d,
            Err(e) => return report_err(&format!("open SurrealDB: {e}")),
        };
        match kavach_surreal::project_set_parent(&db, child, parent).await {
            Ok(()) => {
                let msg = parent.map_or_else(
                    || format!("detached {child} to top-level"),
                    |p| format!("linked {child} -> parent {p}"),
                );
                print_or_exit(&msg).map_or_else(into_exit_code, |()| 0)
            }
            Err(e) => report_err(&format!("{e}")),
        }
    })
}

/// `kavach db tree` — render the project hierarchy with absolute + relative paths.
pub(super) fn render() -> i32 {
    // RPC-first; direct fallback only when the daemon is unreachable.
    // SOURCE: https://github.com/facebook/rocksdb/issues/1780
    match super::rpc_client::tree() {
        Ok(res) => {
            if res.forest.is_empty() {
                return print_or_exit("no projects registered").map_or_else(into_exit_code, |()| 0);
            }
            for root in &res.forest {
                if let Err(code) = print_rpc_node(root, 0, None) {
                    return code;
                }
            }
            return 0;
        }
        Err(e) if super::rpc_client::should_fallback_to_direct(&e) => {}
        Err(e) => return report_err(&e),
    }

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => return report_err(&format!("tokio runtime: {e}")),
    };

    runtime.block_on(async {
        let db = match super::rpc_client::open_direct_resilient().await {
            Ok(d) => d,
            Err(e) => return report_err(&format!("open SurrealDB: {e}")),
        };
        match kavach_surreal::projects_build_forest(&db).await {
            Ok(forest) => {
                if forest.is_empty() {
                    return print_or_exit("no projects registered")
                        .map_or_else(into_exit_code, |()| 0);
                }
                for root in &forest {
                    if let Err(code) = print_node(root, 0, None) {
                        return code;
                    }
                }
                0
            }
            Err(e) => report_err(&format!("{e}")),
        }
    })
}

/// Print one node and recurse into children. `parent_workdir` enables relative-path
/// derivation; roots have none so they show their absolute path only.
fn print_node(node: &ProjectNode, depth: usize, parent_workdir: Option<&str>) -> Result<(), i32> {
    let indent = "  ".repeat(depth);
    let abs = node.project.workdir.as_deref().unwrap_or("(no path)");
    let rel = relative_label(&node.project, parent_workdir);
    let line = format!("{indent}{}{rel} — {abs}", node.project.slug);
    print_or_exit(&line).map_err(into_exit_code)?;

    for child in &node.children {
        print_node(
            child,
            depth.saturating_add(1),
            node.project.workdir.as_deref(),
        )?;
    }
    Ok(())
}

/// Build the ` (rel: <relative>)` suffix when the node nests under a known parent.
fn relative_label(project: &Project, parent_workdir: Option<&str>) -> String {
    match (parent_workdir, project.workdir.as_deref()) {
        (Some(parent), Some(child)) => kavach_surreal::project_relative_to_parent(parent, child)
            .map(|rel| format!(" (rel: {rel})"))
            .unwrap_or_default(),
        _ => String::new(),
    }
}

/// Recursive printer for the RPC `TreeNode` forest. Mirrors `print_node` but
/// over the flat DTO; relative paths are derived CLI-side from the parent's
/// workdir, identical to the direct path.
fn print_rpc_node(
    node: &kavach_rpc::methods::db::TreeNode,
    depth: usize,
    parent_workdir: Option<&str>,
) -> Result<(), i32> {
    let indent = "  ".repeat(depth);
    let abs = node.workdir.as_deref().unwrap_or("(no path)");
    let rel = match (parent_workdir, node.workdir.as_deref()) {
        (Some(parent), Some(child)) => kavach_surreal::project_relative_to_parent(parent, child)
            .map(|r| format!(" (rel: {r})"))
            .unwrap_or_default(),
        _ => String::new(),
    };
    let line = format!("{indent}{}{rel} — {abs}", node.slug);
    print_or_exit(&line).map_err(into_exit_code)?;
    for child in &node.children {
        print_rpc_node(child, depth.saturating_add(1), node.workdir.as_deref())?;
    }
    Ok(())
}

fn report_err(msg: &str) -> i32 {
    ewrite_or_exit(&format!("error: {msg}")).map_or_else(into_exit_code, |()| 1)
}
