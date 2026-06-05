//! Emit path: budget circuit-breaker → severity dispatch → advisory stash.
use kavach_hook::GateSeverity;
use kavach_session::SessionState;

use super::budget::{
    PER_CALL_BUDGET, PER_TURN_BUDGET, call_budget_exhausted, turn_budget_exhausted,
};

/// Emit a gate decision. Returns `true` iff the dispatched exit blocked.
#[expect(
    clippy::print_stderr,
    reason = "hook engine has no tracing dep; stderr is the hook log channel"
)]
pub(crate) fn emit(
    session: &mut SessionState,
    severity: GateSeverity,
    gate_name: &str,
    reason: &str,
) -> bool {
    session.gates_fired_this_turn = session.gates_fired_this_turn.saturating_add(1);
    session.gates_fired_this_call = session.gates_fired_this_call.saturating_add(1);
    if call_budget_exhausted(session) {
        eprintln!(
            "[GATE_OVERLOAD] gate={gate_name} would fire {severity:?} but call \
             budget exhausted ({}/{PER_CALL_BUDGET}); downgrading to silent",
            session.gates_fired_this_call
        );
        drop(kavach_hook::exit_silent());
        return false;
    }
    if turn_budget_exhausted(session) {
        eprintln!(
            "[GATE_OVERLOAD] gate={gate_name} would fire {severity:?} but turn \
             budget exhausted ({}/{PER_TURN_BUDGET}); downgrading to silent",
            session.gates_fired_this_turn
        );
        drop(kavach_hook::exit_silent());
        return false;
    }
    if severity == GateSeverity::P2Advise {
        record_advisory(session, gate_name, reason);
    }
    dispatch(severity, gate_name, reason)
}

fn dispatch(severity: GateSeverity, gate_name: &str, reason: &str) -> bool {
    match severity {
        GateSeverity::P0Block => {
            drop(kavach_hook::exit_pre_tool_deny(&format!(
                "[GATE:{gate_name}] {reason}"
            )));
            true
        }
        GateSeverity::P1Ask => {
            drop(kavach_hook::exit_pre_tool_ask(&format!(
                "[GATE:{gate_name}] {reason}"
            )));
            false
        }
        GateSeverity::P2Advise => {
            drop(kavach_hook::exit_pre_tool_allow(Some(&format!(
                "[ADVISORY:{gate_name}] {reason}"
            ))));
            false
        }
    }
}

/// Stash a `P2Advise` gate-fire on the session so `post_tool` can re-surface it
/// as `[ADVISORY_RECOVERY:<gate>]` next turn. SOURCE: roadmap.unit.agent-feedback-loop.
fn record_advisory(session: &mut SessionState, gate_name: &str, fix: &str) {
    session.last_advisory_gate.clear();
    session.last_advisory_gate.push_str(gate_name);
    session.last_advisory_fix.clear();
    session.last_advisory_fix.push_str(fix);
    session.save().ok();
}
