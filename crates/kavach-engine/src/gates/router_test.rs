//! Budget circuit-breaker invariants: cap boundaries, tool-call/turn resets.
use kavach_session::SessionState;

use super::budget::{
    PER_CALL_BUDGET, PER_TURN_BUDGET, call_budget_exhausted, observe_tool_call, reset_for_new_turn,
    turn_budget_exhausted,
};

#[test]
fn turn_budget_not_exhausted_at_cap() {
    let mut s = SessionState::default();
    s.gates_fired_this_turn = PER_TURN_BUDGET;
    assert!(!turn_budget_exhausted(&s));
}

#[test]
fn turn_budget_exhausted_above_cap() {
    let mut s = SessionState::default();
    s.gates_fired_this_turn = PER_TURN_BUDGET + 1;
    assert!(turn_budget_exhausted(&s));
}

#[test]
fn call_budget_exhausted_above_cap() {
    let mut s = SessionState::default();
    s.gates_fired_this_call = PER_CALL_BUDGET + 1;
    assert!(call_budget_exhausted(&s));
}

#[test]
fn observe_resets_on_new_tool_use_id() {
    let mut s = SessionState::default();
    s.last_seen_tool_use_id = "old-id".to_owned();
    s.gates_fired_this_call = 5;
    observe_tool_call(&mut s, "new-id");
    assert_eq!(s.gates_fired_this_call, 0);
    assert_eq!(s.last_seen_tool_use_id, "new-id");
}

#[test]
fn observe_no_reset_on_same_tool_use_id() {
    let mut s = SessionState::default();
    s.last_seen_tool_use_id = "same".to_owned();
    s.gates_fired_this_call = 2;
    observe_tool_call(&mut s, "same");
    assert_eq!(s.gates_fired_this_call, 2);
}

#[test]
fn observe_no_reset_on_empty_id() {
    let mut s = SessionState::default();
    s.last_seen_tool_use_id = "real-id".to_owned();
    s.gates_fired_this_call = 1;
    observe_tool_call(&mut s, "");
    assert_eq!(s.gates_fired_this_call, 1);
    assert_eq!(s.last_seen_tool_use_id, "real-id");
}

#[test]
fn reset_zeroes_turn_counter() {
    let mut s = SessionState::default();
    s.gates_fired_this_turn = 7;
    reset_for_new_turn(&mut s);
    assert_eq!(s.gates_fired_this_turn, 0);
}

#[test]
fn budget_constants_sane() {
    #[expect(
        clippy::assertions_on_constants,
        reason = "intentional: verify constant relationships at test time"
    )]
    {
        assert!(PER_CALL_BUDGET > 0);
        assert!(PER_TURN_BUDGET >= PER_CALL_BUDGET);
    }
}
