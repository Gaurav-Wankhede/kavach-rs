use kavach_types::HookInput;

/// `PreToolUse:AskUserQuestion` gate — P0 hard block on the labor-as-direction
/// loophole: an `AskUserQuestion` whose `(Recommended)` option is the lower-effort
/// "leave as-is / skip / later / defer" path while a sibling is the do-the-work
/// option. The user decides direction; the agent does ALL the labor (the global
/// division-of-labor rule). FP-bound proven in
/// `kavach-patterns/src/laziness_guard_test.rs`: fires only on an effort split,
/// never on a genuine direction question.
pub(crate) fn handle_question(input: &HookInput) {
    // Claude-Code only: check vendor before applying laziness gate.
    // See decision.engine.vendor_scoped_laziness_gate.
    if vendor_is_exempt(kavach_hook::output_vendor()) {
        let mut session = kavach_session::get_or_create_session();
        super::turn_relay::exit_pre_tool_allow_relay(&mut session, None);
        return;
    }
    // Fail-closed: deny if tool_input is absent (cannot validate questions).
    // See decision.engine.fail_closed_missing_tool_input.
    let Some(map) = input.tool_input.as_ref() else {
        drop(kavach_hook::exit_pre_tool_deny(
            "[LAZINESS_BLOCK] AskUserQuestion arrived with no tool_input — its options \
             cannot be validated against the labor-as-direction rule. Re-issue the question \
             with explicit options, or just DO the work instead of asking.",
        ));
        return;
    };
    // `tool_input` is a flat HashMap; the detector reads the `questions` array via
    // the serde_json Value API, so lift the map into a Value::Object once. A single
    // allocation per AskUserQuestion call — negligible — and it keeps the detector
    // pure (Value in, Option<String> out) and unit-testable.
    let value = serde_json::Value::Object(map.clone().into_iter().collect());
    if let Some(reason) = kavach_patterns::laziness_guard::detect_lazy_recommendation(&value) {
        drop(kavach_hook::exit_pre_tool_deny(&reason));
        return;
    }
    // Researchable-question nudge (ADVISORY): fact-based questions routed to WebSearch.
    // SOURCE: decision.engine.researchable_question_advisory.
    let mut session = kavach_session::get_or_create_session();
    let research_ctx = kavach_patterns::laziness_guard::detect_researchable_question(&value);
    super::turn_relay::exit_pre_tool_allow_relay(&mut session, research_ctx.as_deref());
}

/// `true` when `vendor` is exempt from the laziness rule. Only Claude Code is
/// governed by the division-of-labor directive; every other harness (Cursor,
/// Codex, Antigravity, Pi) runs a different agent under a different policy. The
/// unset thread-local defaults to [`Vendor::ClaudeCode`], so an unknown/ambiguous
/// vendor is NOT exempt — the gate fails toward firing (the protective default),
/// never toward a silent skip.
fn vendor_is_exempt(vendor: kavach_hook::Vendor) -> bool {
    vendor != kavach_hook::Vendor::ClaudeCode
}

#[cfg(test)]
mod tests {
    use super::vendor_is_exempt;
    use kavach_hook::Vendor;

    #[test]
    fn claude_code_is_governed_not_exempt() {
        // The canonical/default vendor MUST run the laziness gate — incl. the
        // unset thread-local case, which resolves to ClaudeCode.
        assert!(!vendor_is_exempt(Vendor::ClaudeCode));
    }

    #[test]
    fn every_other_vendor_is_exempt() {
        // Cursor (Composer 2.5) is the motivating case; the exemption applies to
        // every non-Claude-Code harness that spawns this binary.
        for v in [Vendor::Cursor, Vendor::Codex, Vendor::Antigravity, Vendor::Pi] {
            assert!(vendor_is_exempt(v), "{} must be exempt", v.name());
        }
    }
}
