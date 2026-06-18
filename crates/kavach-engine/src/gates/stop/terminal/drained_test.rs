use super::{
    all_blocked_context, board_drained_plan_context, census_is_all_blocked, cycle_deadlock_context,
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
fn cycle_deadlock_context_refuses_stop_and_directs_the_fix() {
    let c = cycle_deadlock_context();
    assert!(c.contains("CYCLE_DEADLOCK"), "tag present: {c}");
    assert!(c.contains("Do NOT stop"), "refuses the clean stop: {c}");
    assert!(c.contains("mermaid"), "points at the cycle view: {c}");
}

#[test]
fn all_blocked_context_directs_dependency_first_resolution_not_user_handoff() {
    let c = all_blocked_context(Some((2, 2, 0)));
    assert!(c.contains("ALL_BLOCKED"), "tag present: {c}");
    assert!(
        c.contains("dependency"),
        "names the prerequisite class: {c}"
    );
    // The bug fix: a dependency block is AI-repairable work, NOT a user hand-off.
    // The verdict must direct the agent to WALK to the blocker and BUILD it, and
    // must NOT defer the unblock to the user.
    assert!(
        c.contains("Do NOT hand the unblock to the user"),
        "refuses the user hand-off (the reported bug): {c}"
    );
    assert!(
        c.contains("BUILD the blocker"),
        "directs dependency-first build of the blocker: {c}"
    );
    assert!(
        c.contains("WALK to the blocking card"),
        "directs a walk to the blocking card (leaf-first): {c}"
    );
    assert!(
        c.contains("STALE/FALSE"),
        "directs correcting a stale/false dependency edge: {c}"
    );
    assert!(
        c.contains("owner-only"),
        "reserves escalation for genuinely owner-only blockers only: {c}"
    );
    // The verdict must STAMP the census it read (verdict_needs_leaf_evidence):
    // the live counts + proof the gate read the DB this stop.
    assert!(
        c.contains("census:") && c.contains("runnable=2 blocked=2 cyclic=0"),
        "stamps the live census it read: {c}"
    );
    assert!(
        c.contains("read the kavach DB roadmap table this stop"),
        "cites the leaf it read this stop: {c}"
    );
    assert!(
        c.contains("do NOT re-run `kavach db kanban`"),
        "tells the AI not to redundantly re-query what the gate already read: {c}"
    );
    // The loop never self-terminates — even all-blocked re-scans the DB and
    // yields to the user's Esc, never a hardcoded clean stop.
    assert!(c.contains("Do NOT stop"), "refuses to self-terminate: {c}");
    assert!(c.contains("Esc"), "yields only to the user halt: {c}");
    assert!(
        !c.contains("Clean stop") && !c.contains("clean stop"),
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
    // never trust training weights. Both next-task verdicts carry the directive.
    for c in [board_drained_plan_context(Some((0, 0, 0))), all_blocked_context(Some((1, 1, 0)))] {
        assert!(c.contains("RESEARCH MODE"), "names Research Mode: {c}");
        assert!(c.contains("WebSearch"), "directs an internet search: {c}");
        assert!(
            c.contains("TABULA RASA = TRUTH"),
            "asserts truth over weights: {c}"
        );
        assert!(
            c.contains("NEVER trust training weights"),
            "forbids answering from weights: {c}"
        );
    }
}
