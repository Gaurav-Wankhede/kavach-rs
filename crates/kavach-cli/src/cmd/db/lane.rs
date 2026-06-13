// `kavach db lane-set` — pin a roadmap card to a dispatch LANE for affinity
// sharding. Routes through kavach_surreal::set_lane (db.set_lane RPC). Touches
// ONLY lane + updated_at; title/content/status/priority preserved.

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

const ROADMAP: &str = "roadmap";

pub(super) fn run(project_slug: &str, key: &str, new_lane: Option<String>, clear: bool) -> i32 {
    if !clear && new_lane.is_none() {
        return write_err("error: must specify --lane <name> or --clear");
    }
    let effective: Option<String> = if clear { None } else { new_lane };

    match super::rpc_client::set_lane(project_slug, ROADMAP, key, effective.clone()) {
        Ok(result) if result.success => print_pretty(key, effective.as_deref()),
        Ok(result) => write_err(&format!(
            "error: {}",
            result.error.unwrap_or_else(|| "unknown".to_owned())
        )),
        Err(e) => write_err(&format!("rpc error: {e}")),
    }
}

fn print_pretty(key: &str, effective: Option<&str>) -> i32 {
    let msg = effective.map_or_else(
        || format!("lane cleared: [roadmap] {key} → UNLANED"),
        |lane| format!("lane set: [roadmap] {key} → {lane}"),
    );
    if let Err(io_err) = print_or_exit(&msg) {
        return into_exit_code(io_err);
    }
    0
}

fn write_err(msg: &str) -> i32 {
    if let Err(io_err) = ewrite_or_exit(msg) {
        return into_exit_code(io_err);
    }
    1
}
