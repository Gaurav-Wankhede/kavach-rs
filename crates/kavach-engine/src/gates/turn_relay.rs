//! Cursor turn-shadow + post-tool advisory relay flush (Phases 1 & 5).
use kavach_hook::Vendor;
use kavach_session::{RelayFlush, SessionState};

/// True when the active harness is Cursor (the only vendor that drops allow-path context).
#[must_use]
pub(crate) fn should_relay() -> bool {
    kavach_hook::output_vendor() == Vendor::Cursor
}

/// Merge pending relay payload ahead of optional gate context.
#[must_use]
pub(crate) fn merge_relay(
    session: &mut SessionState,
    ctx: Option<String>,
    flush: RelayFlush,
) -> Option<String> {
    if !should_relay() {
        return ctx;
    }
    let Some(relay) = session.take_relay_payload(flush) else {
        return ctx;
    };
    Some(match ctx {
        Some(existing) if existing.is_empty() => relay,
        Some(existing) => format!("{relay}\n\n{existing}"),
        None => relay,
    })
}

/// Queue a one-line advisory for the next relay flush (Cursor only).
pub(crate) fn queue_advisory(session: &mut SessionState, line: &str) {
    if should_relay() {
        session.queue_pending_advisory(line);
    }
}

/// `PreToolUse` allow exit — advisories only; defer turn shadow until pre-write.
pub(crate) fn exit_pre_tool_allow_relay(session: &mut SessionState, ctx: Option<&str>) {
    let merged = merge_relay(session, ctx.map(str::to_owned), RelayFlush::AdvisoriesOnly);
    drop(kavach_hook::exit_pre_tool_allow(merged.as_deref()));
}

/// `PreToolUse` deny exit — enforce block action from rule engine.
pub(crate) fn exit_pre_tool_deny(reason: &str) {
    drop(kavach_hook::exit_pre_tool_deny(reason));
}

/// `PreWrite` allow exit — full shadow + advisories at point-of-action.
pub(crate) fn exit_pre_write_allow_relay(session: &mut SessionState, ctx: Option<&str>) {
    let merged = merge_relay(session, ctx.map(str::to_owned), RelayFlush::Full);
    drop(kavach_hook::exit_pre_tool_allow(merged.as_deref()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use kavach_hook::Vendor;

    #[test]
    fn merge_relay_full_prepends_shadow_on_cursor() {
        kavach_hook::set_output_context(Vendor::Cursor, "PreToolUse");
        let mut session = SessionState::default();
        session.store_turn_shadow("[INTENT] type:fix");
        let merged = merge_relay(
            &mut session,
            Some("tail".to_owned()),
            RelayFlush::Full,
        )
        .expect("merged");
        assert!(merged.starts_with("[INTENT]"));
        assert!(merged.contains("tail"));
        assert!(!session.turn_shadow_pending());
    }

    #[test]
    fn merge_relay_advisories_only_keeps_shadow_pending() {
        kavach_hook::set_output_context(Vendor::Cursor, "PreToolUse");
        let mut session = SessionState::default();
        session.store_turn_shadow("[INTENT] type:fix");
        let merged = merge_relay(&mut session, Some("tail".to_owned()), RelayFlush::AdvisoriesOnly);
        assert_eq!(merged.as_deref(), Some("tail"));
        assert!(session.turn_shadow_pending());
    }

    #[test]
    fn merge_relay_noop_on_claude_code() {
        kavach_hook::set_output_context(Vendor::ClaudeCode, "PreToolUse");
        let mut session = SessionState::default();
        session.store_turn_shadow("[INTENT] type:fix");
        let merged = merge_relay(
            &mut session,
            Some("only".to_owned()),
            RelayFlush::Full,
        );
        assert_eq!(merged.as_deref(), Some("only"));
        assert!(session.turn_shadow_pending());
    }
}
