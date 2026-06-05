use kavach_types::HookInput;

/// Stop-failure gate: handle the stop-failure hook event.
///
/// No-op behavior-wise — Claude Code's stop-failure event carries no payload
/// that this engine needs to act on; downstream telemetry is recorded by
/// `post_tool_failure`. Consumes `input` for an audit-trail diagnostic so the
/// gate firing is observable in stderr-tailed harness logs.
#[expect(
    clippy::print_stderr,
    reason = "hook engine has no tracing dep; stderr is the hook log channel"
)]
pub(crate) fn run(input: &HookInput) {
    eprintln!(
        "[STOP_FAILURE] gate invoked session={} tool={} (no-op; failure path in post_tool_failure)",
        input.session_id, input.tool_name
    );
    drop(kavach_hook::exit_silent());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_run() {
        run(&HookInput::default());
    }
}
