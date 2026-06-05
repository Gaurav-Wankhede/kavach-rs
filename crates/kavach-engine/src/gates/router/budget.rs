//! Per-call + per-turn gate-fire budget: counters, exhaustion checks, and the
//! tool-call / turn boundary resets that drive the circuit-breaker.
use kavach_session::SessionState;

/// Max gates that may fire on a single tool call before downgrade.
pub(super) const PER_CALL_BUDGET: i32 = 3;
/// Max gates that may fire across a whole turn before downgrade.
pub(super) const PER_TURN_BUDGET: i32 = 10;

pub(super) const fn turn_budget_exhausted(session: &SessionState) -> bool {
    session.gates_fired_this_turn > PER_TURN_BUDGET
}

pub(super) const fn call_budget_exhausted(session: &SessionState) -> bool {
    session.gates_fired_this_call > PER_CALL_BUDGET
}

/// Reset the per-call counter when the `tool_use_id` changes. Call at the top of
/// every gate entry-point before `emit()` so the call-scoped budget tracks real
/// tool-call boundaries (not gate-internal sub-checks). Keyed on `tool_use_id`.
pub(crate) fn observe_tool_call(session: &mut SessionState, tool_use_id: &str) {
    if !tool_use_id.is_empty() && session.last_seen_tool_use_id != tool_use_id {
        session.last_seen_tool_use_id.clear();
        session.last_seen_tool_use_id.push_str(tool_use_id);
        session.gates_fired_this_call = 0;
    }
}

/// Reset the per-turn counter at turn boundary.
pub(crate) const fn reset_for_new_turn(session: &mut SessionState) {
    session.gates_fired_this_turn = 0;
}
