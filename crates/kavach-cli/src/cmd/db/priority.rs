// `kavach db priority-set` — surgical priority rerank for human-in-loop focus.
// Routes through kavach_surreal::set_priority. Touches ONLY priority +
// updated_at; title/content/status preserved.
mod direct;

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(super) fn run(
    project_slug: &str,
    category: &str,
    key: &str,
    new_priority: Option<i64>,
    clear: bool,
) -> i32 {
    if !clear && new_priority.is_none() {
        return write_err("error: must specify --priority N or --clear");
    }
    let effective: Option<i64> = if clear { None } else { new_priority };

    if category != "roadmap" && category != "decision" {
        return write_err(&format!(
            "error: priority is only defined on roadmap and decision tables, got: {category}"
        ));
    }

    match super::rpc_client::set_priority(project_slug, category, key, effective) {
        Ok(result) if result.success => {
            return print_pretty(category, key, effective, true);
        }
        Ok(result) => {
            return write_err(&format!(
                "error: {}",
                result.error.unwrap_or_else(|| "unknown".to_owned())
            ));
        }
        Err(e) if super::rpc_client::should_fallback_to_direct(&e) => {}
        Err(e) => {
            return write_err(&format!("rpc error: {e}"));
        }
    }

    direct::run_direct(project_slug, category, key, effective)
}

pub(super) fn print_pretty(
    category: &str,
    key: &str,
    effective: Option<i64>,
    via_rpc: bool,
) -> i32 {
    let suffix = if via_rpc { " (via rpc)" } else { "" };
    let msg = effective.map_or_else(
        || format!("priority cleared: [{category}] {key} → NONE{suffix}"),
        |n| format!("priority set: [{category}] {key} → {n}{suffix}"),
    );
    if let Err(io_err) = print_or_exit(&msg) {
        return into_exit_code(io_err);
    }
    0
}

pub(super) fn write_err(msg: &str) -> i32 {
    if let Err(io_err) = ewrite_or_exit(msg) {
        return into_exit_code(io_err);
    }
    1
}
