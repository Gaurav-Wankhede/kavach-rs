// `kavach goal stop` — terminate a goal row + emit `/goal clear`.
// SOURCE: roadmap.unit.kavach-goal-bridge.
use crate::cmd::io_safe::{into_exit_code, print_or_exit};
use serde_json::json;

pub(crate) fn run(project: &str, condition: &str) -> i32 {
    let key = format!("goal.{}", super::start::slugify_for_test(condition));
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
        eprintln!("kavach goal stop: rpc db.status_update: {e}");
        return 1;
    }
    let banner = format!(
        "[GOAL_CLEARED] project={project} condition={condition:?}\n\n\
         PASTE INTO CLAUDE CODE NOW (if /goal still active):\n  \
         /goal clear"
    );
    if let Err(e) = print_or_exit(&banner) {
        return into_exit_code(e);
    }
    0
}
