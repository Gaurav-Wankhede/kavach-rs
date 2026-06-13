use super::{all_blocked_context, board_drained_plan_context, census_is_all_blocked};

#[test]
fn lone_blocked_card_is_all_blocked() {
    // The reported bug: one todo card, blocked on Windows CI → clean stop.
    assert!(census_is_all_blocked(Some((1, 1))));
}

#[test]
fn every_remaining_card_blocked_is_all_blocked() {
    assert!(census_is_all_blocked(Some((3, 3))));
}

#[test]
fn some_runnable_some_blocked_is_not_all_blocked() {
    // A dispatchable card exists — defer to the nudge (real work remains).
    assert!(!census_is_all_blocked(Some((3, 2))));
}

#[test]
fn empty_board_is_not_all_blocked() {
    // Zero runnable cards → PLAN nudge, not an ALL_BLOCKED stop.
    assert!(!census_is_all_blocked(Some((0, 0))));
}

#[test]
fn rpc_outage_fails_closed_to_nudge() {
    // None = census unobservable → never a wrong clean-stop.
    assert!(!census_is_all_blocked(None));
}

#[test]
fn all_blocked_context_names_the_owner_gate() {
    let c = all_blocked_context();
    assert!(c.contains("ALL_BLOCKED"), "tag present: {c}");
    assert!(
        c.contains("owner-gate"),
        "names the prerequisite class: {c}"
    );
}

#[test]
fn plan_context_nudges_instead_of_silent_stop() {
    // The fix's core: a drained board emits the PLAN nudge, never silence.
    let c = board_drained_plan_context();
    assert!(c.contains("AUTO_CONTINUE"), "continue tag present: {c}");
    assert!(
        c.contains("un-built next phase"),
        "names the un-built work: {c}"
    );
    assert!(
        c.contains("genuine clean stop"),
        "still allows a real stop when the plan is fully built: {c}"
    );
}
