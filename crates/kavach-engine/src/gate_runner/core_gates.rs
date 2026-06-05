//! Dispatch for the write/tool/intent gate family + the always-Ok side-effect
//! gates (permission/notification/message-display/stop-failure/pre-tool-search).
use kavach_types::HookInput;

use super::util::ok;
use crate::error::EngineError;
use crate::gates;

/// Match a core write/tool/intent gate. Returns `None` to fall through to the
/// next family.
pub(super) fn dispatch(gate_name: &str, input: &HookInput) -> Option<Result<(), EngineError>> {
    let result = match gate_name {
        "pre-write" => gates::pre_write::run(input),
        "post-write" => gates::post_write::run(input),
        "pre-tool" => gates::pre_tool::run(input),
        "post-tool" => gates::post_tool::run(input),
        "intent" => gates::intent::run(input),
        "teammate-idle" => gates::teammate::run_idle(input),
        "task-completed" => gates::teammate::run_task_completed(input),
        "post-tool-failure" => gates::post_tool_failure::run(input),
        "stop" => gates::stop::run(input),
        "permission" => ok(|| gates::permission::run(input)),
        "notification" => ok(|| gates::notification::run(input)),
        "message-display" => ok(|| gates::message_display::run(input)),
        "stop-failure" => ok(|| gates::stop_failure::run(input)),
        "pre-tool-search" => ok(|| gates::pre_tool_search::run(input)),
        _ => return None,
    };
    Some(result)
}
