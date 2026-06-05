use kavach_types::HookInput;

use crate::error::EngineError;

/// Handle task done: track subagent lifecycle.
/// Subagent telemetry is recorded by `subagent::run_stop`; this hook marks the
/// lifecycle close on stderr so the audit trail covers PostToolUse:Task too.
#[expect(
    clippy::print_stderr,
    reason = "hook engine has no tracing dep; stderr is the hook log channel"
)]
#[expect(
    clippy::unnecessary_wraps,
    reason = "signature fixed by the post_tool::run match dispatch: every per-tool handler returns Result<(), EngineError>"
)]
pub(crate) fn handle_task_done(input: &HookInput) -> Result<(), EngineError> {
    eprintln!(
        "[POST_TOOL:TASK] task complete session={} tool={}",
        input.session_id, input.tool_name
    );
    drop(kavach_hook::exit_silent());
    Ok(())
}

/// PostToolUse:Task dispatch from `post_tool::run`. Session state is consumed
/// for the turn-correlated audit line; behavior delegates to `handle_task_done`.
#[expect(
    clippy::print_stderr,
    reason = "hook engine has no tracing dep; stderr is the hook log channel"
)]
pub(crate) fn handle(
    input: &HookInput,
    session: &kavach_session::SessionState,
) -> Result<(), EngineError> {
    eprintln!(
        "[POST_TOOL:TASK] dispatch turn={} phase={}",
        session.turn_count, session.current_phase
    );
    handle_task_done(input)
}
