use super::{
    blocker_walk_context, board_drained_plan_context, census_has_dispatchable_remainder,
    census_is_all_blocked,
};

#[test]
fn lone_blocked_card_is_all_blocked() {
    // The reported bug: one todo card, blocked on Windows CI → clean stop.
    assert!(census_is_all_blocked(Some((1, 1, 0))));
}

#[test]
fn every_remaining_card_blocked_is_all_blocked() {
    assert!(census_is_all_blocked(Some((3, 3, 0))));
}

#[test]
fn blocked_plus_cyclic_covering_runnable_is_all_blocked() {
    // 2 blocked + 1 cyclic == 3 runnable. The cycle is handled earlier by
    // drained_terminal_context; this predicate still treats the set as drained.
    assert!(census_is_all_blocked(Some((3, 2, 1))));
}

#[test]
fn some_runnable_some_blocked_is_not_all_blocked() {
    // A dispatchable card exists — defer to the nudge (real work remains).
    assert!(!census_is_all_blocked(Some((3, 2, 0))));
}

#[test]
fn empty_board_is_not_all_blocked() {
    // Zero runnable cards → PLAN nudge, not an ALL_BLOCKED stop.
    assert!(!census_is_all_blocked(Some((0, 0, 0))));
}

#[test]
fn rpc_outage_fails_closed_to_nudge() {
    // None = census unobservable → never a wrong clean-stop.
    assert!(!census_is_all_blocked(None));
}

#[test]
fn dispatchable_remainder_fires_when_runnable_exceeds_blocked_plus_cyclic() {
    // The reported real-world bug: census runnable=21 blocked=0 cyclic=0, yet
    // dispatch returned None — runnable, UNBLOCKED roadmap todos remain. The gate
    // must REFUSE the clean-stop, not nudge.
    assert!(census_has_dispatchable_remainder(Some((21, 0, 0))));
    assert!(census_has_dispatchable_remainder(Some((3, 2, 0)))); // 1 dispatchable
}

#[test]
fn dispatchable_remainder_silent_when_all_blocked_or_empty_or_outage() {
    // Every runnable card blocked/cyclic → the all-blocked path owns it, not this.
    assert!(!census_has_dispatchable_remainder(Some((3, 3, 0))));
    assert!(!census_has_dispatchable_remainder(Some((3, 2, 1))));
    // Empty board → drained-plan nudge, not a refuse-stop.
    assert!(!census_has_dispatchable_remainder(Some((0, 0, 0))));
    // RPC outage → false here (the outage nudge owns fail-closed elsewhere).
    assert!(!census_has_dispatchable_remainder(None));
}

#[test]
fn blocker_walk_context_refuses_stop_and_directs_dependency_first_build() {
    // [ALL_BLOCKED] is abolished: a fully-blocked board (deps OR cycle) is a single
    // BLOCKER_WALK directive — WALK to the blocker and BUILD it, never a clean stop,
    // never a hand-off, never a separate "everything's blocked" terminal tag.
    let c = blocker_walk_context();
    assert!(c.contains("BLOCKER_WALK"), "single blocker-walk tag present: {c}");
    assert!(!c.contains("ALL_BLOCKED"), "the abolished tag is gone: {c}");
    assert!(c.contains("Do NOT stop"), "refuses the clean stop: {c}");
    assert!(c.contains("BUILD the blocker"), "directs dependency-first build: {c}");
    assert!(c.contains("CYCLE"), "folds in the cycle-break directive: {c}");
    assert!(c.contains("mermaid"), "points at the cycle view for a cycle: {c}");
    assert!(c.contains("STALE/FALSE"), "directs correcting a stale edge: {c}");
    assert!(
        c.contains("runtime script") && c.contains("dotenvy"),
        "secret-bound ops go via a runtime script, never a hand-back: {c}"
    );
    assert!(
        c.contains("genuinely ABSENT") && c.contains("FILED"),
        "files a card only when the env var is genuinely absent: {c}"
    );
    assert!(c.contains("Esc"), "yields only to the user halt: {c}");
    assert!(
        !c.contains("clean stop") && !c.contains("Clean stop"),
        "carries no clean-stop language: {c}"
    );
}

#[test]
fn census_stamp_marks_rpc_outage_explicitly_so_no_false_read_claim() {
    // None = RPC outage. The verdict must NOT claim a read it could not make;
    // it must say the board was unobservable and the backlog is non-empty.
    let c = board_drained_plan_context(None);
    assert!(
        c.contains("UNOBSERVABLE") && c.contains("treat the backlog as non-empty"),
        "an unobservable board is stamped, never silently claimed read: {c}"
    );
}

#[test]
fn plan_context_directs_db_rescan_and_never_self_stops() {
    // The contract: a drained board re-scans the DB (roadmap + decisions, all
    // statuses) for the next task and never tells the LLM to stop. Only the
    // user halts the loop, with Esc.
    let c = board_drained_plan_context(Some((0, 0, 0)));
    assert!(c.contains("AUTO_CONTINUE"), "continue tag present: {c}");
    assert!(c.contains("do NOT stop"), "never self-terminates: {c}");
    // The drained verdict stamps the empty census it read (verdict_needs_leaf_evidence).
    assert!(
        c.contains("census:") && c.contains("runnable=0 blocked=0 cyclic=0"),
        "stamps the empty census it read this stop: {c}"
    );
    assert!(
        c.contains("--category roadmap") && c.contains("--category decision"),
        "directs DB scan across roadmap AND decisions: {c}"
    );
    assert!(
        c.contains("Esc"),
        "the loop yields only to the user halt: {c}"
    );
    assert!(
        !c.contains("clean stop") && !c.contains("clean-stop"),
        "no hardcoded clean-stop language remains: {c}"
    );
}

#[test]
fn next_task_verdicts_mandate_research_mode_against_truth() {
    // The built-in Stop→next-task step is research-first: WebSearch current truth,
    // never trust training weights. The drained-plan next-task verdict carries it.
    let c = board_drained_plan_context(Some((0, 0, 0)));
    assert!(c.contains("RESEARCH MODE"), "names Research Mode: {c}");
    assert!(c.contains("WebSearch"), "directs an internet search: {c}");
    assert!(c.contains("TABULA RASA = TRUTH"), "asserts truth over weights: {c}");
    assert!(c.contains("NEVER trust training weights"), "forbids answering from weights: {c}");
}
