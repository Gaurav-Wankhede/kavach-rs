//! Canary SHADOW mode (harness-rl Wave P4): log what the RSCB-MC controller
//! WOULD decide vs what the rule gate actually did — without ever acting on it.
//!
//! Gated entirely behind `KAVACH_RL_CANARY`: off (the default), this is a no-op
//! and the controller never runs. On, it records a `shadow_decision` event per
//! ADVISORY gate decision so the divergence between the learned controller and
//! the static rule can be measured offline BEFORE any promotion (design D4).
//!
//! INVARIANTS (fail-closed, design C2/D2):
//! - SHADOW ONLY: this never changes the gate's actual verdict. It observes.
//! - ADVISORY SCOPE: callers pass it only advisory-gate decisions; P0/forbid
//!   gates bypass the controller and never reach here.
//! - The controller defaults to `Ask` (abstention) under uncertainty, so a
//!   shadow that would diverge toward a RISKIER action is the signal to inspect.

use kavach_patterns::bandit_log::GateAction;

#[cfg(test)]
#[path = "shadow_test.rs"]
mod tests;

/// The env flag that arms the canary. Absent/empty/`"0"`/`"false"` ⇒ disarmed.
const CANARY_FLAG: &str = "KAVACH_RL_CANARY";

/// Whether the canary is armed this process (reads `KAVACH_RL_CANARY`).
///
/// Disarmed is the safe default: any value other than a clear truthy
/// (`1`/`true`/`yes`/`on`, case-insensitive) leaves the controller dormant.
#[must_use]
pub(crate) fn canary_armed() -> bool {
    std::env::var(CANARY_FLAG).is_ok_and(|v| is_truthy(&v))
}

/// Parse a flag value as a boolean — only an explicit truthy arms the canary.
fn is_truthy(v: &str) -> bool {
    matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
}

/// Record a shadow decision: what the controller WOULD choose vs the rule's
/// actual action. No-op unless the canary is armed. Fire-and-forget, never
/// blocks the gate, never alters its verdict.
///
/// `session_id` keys the event; `gate`/`tool` identify the advisory gate;
/// `rule_action` is what the gate actually returned; `shadow_action` is what the
/// controller would have done. Logged distinctly from the Layer-A bandit row so
/// shadow observations don't pollute the on-policy training log.
pub(crate) fn record_shadow(
    session_id: &str,
    gate: &str,
    tool: &str,
    rule_action: GateAction,
    shadow_action: GateAction,
) {
    if session_id.is_empty() || !canary_armed() {
        return;
    }
    let params = serde_json::json!({
        "event_type": "shadow_decision",
        "payload": serde_json::json!({
            "session_id": session_id,
            "gate": gate,
            "tool": tool,
            "rule_action": action_str(rule_action),
            "shadow_action": action_str(shadow_action),
            "diverged": rule_action != shadow_action,
        })
        .to_string(),
    });
    // Fire-and-forget: a down daemon must never block or alter the gate.
    #[expect(
        clippy::let_underscore_must_use,
        reason = "fire-and-forget RPC; daemon down is silent-fail by design"
    )]
    let _: Result<serde_json::Value, _> = kavach_rpc::client::call("db.event", Some(params));
}

/// The `snake_case` wire string for a gate action, matching `bandit_log`'s
/// encoding so a shadow event joins the on-policy rows on the same vocabulary.
const fn action_str(action: GateAction) -> &'static str {
    match action {
        GateAction::Allow => "allow",
        GateAction::Ask => "ask",
        GateAction::Block => "block",
        // GateAction is #[non_exhaustive]; a future variant logs as "unknown"
        // rather than failing the fire-and-forget shadow path.
        _ => "unknown",
    }
}
