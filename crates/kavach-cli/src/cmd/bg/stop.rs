// `kavach bg stop` — terminate a bg-session row.
// SOURCE: roadmap.unit.kavach-bg-session.
use crate::cmd::io_safe::{into_exit_code, print_or_exit};
use serde_json::json;

pub(crate) fn run(project: &str, task: &str) -> i32 {
    let key = format!("bg.{}", super::start::slugify_for_test(task));
    let params = json!({
        "project": project,
        "category": "decision",
        "key": key,
        "status": "done",
    });
    if let Err(e) = kavach_rpc::client::call::<serde_json::Value, serde_json::Value>(
        "db.status_update",
        Some(params),
    ) {
        eprintln!("kavach bg stop: rpc db.status_update: {e}");
        return 1;
    }
    let banner = format!("[BG_CLEARED] project={project} task={task:?}");
    if let Err(e) = print_or_exit(&banner) {
        return into_exit_code(e);
    }
    0
}
