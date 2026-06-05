//! Dispatch for the session/worktree/compact lifecycle gate family.
use kavach_types::HookInput;

use super::util::ok;
use crate::error::EngineError;
use crate::gates;

/// Match a session/worktree/compact lifecycle gate. Returns `None` to fall
/// through to the next family.
pub(super) fn dispatch(gate_name: &str, input: &HookInput) -> Option<Result<(), EngineError>> {
    let result = match gate_name {
        "permission-request" => gates::permission_request::run(input),
        "session-start" => gates::session_start::run(input),
        "config-change" => gates::config_change::run(input),
        "worktree-create" => gates::worktree_create::run(input),
        "cwd-changed" => gates::cwd_changed::run(input),
        "file-changed" => gates::file_changed::run(input),
        "task-created" => gates::task_created::run(input),
        "session-end" => gates::session_end::run(input),
        "post-compact" => gates::post_compact::run(input),
        "instructions-loaded" => ok(|| gates::instructions_loaded::run(input)),
        "worktree-remove" => ok(|| gates::worktree_remove::run(input)),
        "pre-compact" => ok(|| gates::pre_compact::run(input)),
        "elicitation" => ok(|| gates::elicitation::run(input)),
        "elicitation-result" => ok(|| gates::elicitation::run_result(input)),
        _ => return None,
    };
    Some(result)
}
