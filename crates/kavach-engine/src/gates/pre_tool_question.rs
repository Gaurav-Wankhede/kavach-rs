use kavach_types::HookInput;

/// `PreToolUse:AskUserQuestion` gate — P0 hard block on the labor-as-direction
/// loophole: an `AskUserQuestion` whose `(Recommended)` option is the lower-effort
/// "leave as-is / skip / later / defer" path while a sibling is the do-the-work
/// option. The user decides direction; the agent does ALL the labor (the global
/// division-of-labor rule). FP-bound proven in
/// `kavach-patterns/src/laziness_guard_test.rs`: fires only on an effort split,
/// never on a genuine direction question.
pub(crate) fn handle_question(input: &HookInput) {
    // VENDOR SCOPE: the laziness rule is a Claude-Code division-of-labor directive
    // (global CLAUDE.md: Cursor runs Composer 2.5 and is exempt from the Kavach
    // harness). Other harnesses spawn the same binary, so the gate would otherwise
    // fire on their AskUserQuestion too. The hook layer already resolved the vendor
    // and stashed it in a thread-local (kavach-hook::set_output_context); reading it
    // here keeps the engine signature vendor-blind while honoring the exemption.
    // Non-Claude-Code vendors skip the laziness check entirely (incl. the fail-closed
    // arm below) — fall through to the normal allow-relay.
    if vendor_is_exempt(kavach_hook::output_vendor()) {
        let mut session = kavach_session::get_or_create_session();
        super::turn_relay::exit_pre_tool_allow_relay(&mut session, None);
        return;
    }
    // FAIL CLOSED: a real AskUserQuestion ALWAYS carries `tool_input.questions`.
    // A call with no `tool_input` cannot be validated, so denying it costs zero
    // legitimate traffic and shuts the fail-open hole where a malformed/absent
    // payload would skip the laziness check entirely (global CLAUDE.md
    // §handle_every_error: deny on uncertainty for anything touching correctness).
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
    let mut session = kavach_session::get_or_create_session();
    super::turn_relay::exit_pre_tool_allow_relay(&mut session, None);
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
