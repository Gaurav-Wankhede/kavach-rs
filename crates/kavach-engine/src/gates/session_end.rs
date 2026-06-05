use kavach_types::HookInput;

use crate::error::EngineError;

/// `SessionEnd` gate: final cleanup and memory sync.
#[expect(
    clippy::unnecessary_wraps,
    reason = "public signature; crates may depend on Result return"
)]
#[expect(
    clippy::print_stderr,
    reason = "hook engine has no tracing dep; stderr is the hook log channel"
)]
pub(crate) fn run(input: &HookInput) -> Result<(), EngineError> {
    let reason = if input.reason.is_empty() {
        "normal"
    } else {
        &input.reason
    };

    let mut session = kavach_session::get_or_create_session();
    session.set_task("session", "ended");
    if let Err(e) = session.save() {
        // Session save failures must not block session-end; surface on stderr
        // so the audit trail captures the lost write.
        eprintln!("[SESSION_END] warning: session save failed: {e}");
    }
    let context = kavach_hook::context_block(
        "SESSION_END",
        &[("why", reason), ("t", &session.turn_count.to_string())],
    );

    super::event_log::log_session(
        &session.session_id,
        "session_end",
        &session.model_id,
        &session.project,
    );

    drop(kavach_hook::exit_session_end(&context));
    Ok(())
}
