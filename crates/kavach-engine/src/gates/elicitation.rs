// Karpathy Principle 1: "Think Before Coding"
// On implement intent with risk >= medium, inject [THINK_FIRST] advisory once per intent window.
use kavach_types::HookInput;

/// Elicitation gate: inject [`THINK_FIRST`] advisory on medium/high-risk implement intents.
///
/// Advisory-only — never blocks. Fires once per intent window via `think_first_injected` flag.
/// The Claude Code elicitation event itself carries no actionable payload here — the
/// advisory is computed and injected upstream in `pre_write_advisory` via
/// `think_first_advisory()`. We still consume `input` to emit a diagnostic so the
/// gate invocation is observable in stderr-tailed harness logs.
#[expect(
    clippy::print_stderr,
    reason = "hook engine has no tracing dep; stderr is the hook log channel"
)]
pub(crate) fn run(input: &HookInput) {
    eprintln!(
        "[ELICITATION] gate invoked session={} tool={} (no-op; advisory lives in pre_write_advisory)",
        input.session_id, input.tool_name
    );
    drop(kavach_hook::exit_silent());
}

/// Elicitation result gate: handle result of an elicitation prompt.
///
/// Currently a no-op — result handling is reserved for future elicitation flows
/// that round-trip user answers. Consumes `input` for the diagnostic stub.
#[expect(
    clippy::print_stderr,
    reason = "hook engine has no tracing dep; stderr is the hook log channel"
)]
pub(crate) fn run_result(input: &HookInput) {
    eprintln!(
        "[ELICITATION_RESULT] gate invoked session={} tool={} (no-op; result flow not yet wired)",
        input.session_id, input.tool_name
    );
    drop(kavach_hook::exit_silent());
}

/// Returns [`THINK_FIRST`] advisory string if conditions are met, None otherwise.
/// Called from `pre_write` gate section 8 (advisory zone).
#[must_use]
pub(crate) fn think_first_advisory(
    intent_type: &str,
    intent_risk: &str,
    already_injected: bool,
) -> Option<String> {
    if already_injected {
        return None;
    }
    if intent_type != "implement" {
        return None;
    }
    if intent_risk == "low" {
        return None;
    }
    Some(
        "[THINK_FIRST]\n\
         action: State assumptions explicitly. If multiple interpretations exist, present them.\n\
         action: If a simpler approach exists, say so. If unclear, ask — don't guess.\n"
            .to_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run() {
        run(&HookInput::default());
    }

    #[test]
    fn test_run_result() {
        run_result(&HookInput::default());
    }

    #[test]
    fn should_inject_on_implement_medium_risk() {
        let result = think_first_advisory("implement", "medium", false);
        assert!(result.is_some());
        let s = result.unwrap_or_default();
        assert!(s.contains("[THINK_FIRST]"));
    }

    #[test]
    fn should_not_inject_when_already_done() {
        assert!(think_first_advisory("implement", "medium", true).is_none());
    }

    #[test]
    fn should_not_inject_for_low_risk() {
        assert!(think_first_advisory("implement", "low", false).is_none());
    }

    #[test]
    fn should_not_inject_for_non_implement_intent() {
        assert!(think_first_advisory("refactor", "high", false).is_none());
        assert!(think_first_advisory("debug", "medium", false).is_none());
        assert!(think_first_advisory("general", "high", false).is_none());
    }

    #[test]
    fn should_inject_on_high_risk_implement() {
        let result = think_first_advisory("implement", "high", false);
        assert!(result.is_some());
    }
}
