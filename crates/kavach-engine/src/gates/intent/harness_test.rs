use super::*;

#[test]
fn classify_routes_each_pattern_by_keyword() {
    assert_eq!(
        classify_harness("triage these incoming bug reports"),
        "classify-act"
    );
    assert_eq!(
        classify_harness("audit every handler across the workspace"),
        "fan-out-synthesize"
    );
    assert_eq!(
        classify_harness("review and adversarially verify this fix"),
        "worker-critic"
    );
    assert_eq!(
        classify_harness("brainstorm several candidate designs"),
        "generate-filter"
    );
    assert_eq!(
        classify_harness("compare these and pick the winner"),
        "pairwise-tournament"
    );
}

#[test]
fn classify_defaults_to_loop_until_done() {
    // Open-ended build/fix work with no routing keyword keeps the original
    // goal-loop behavior — nothing regresses.
    assert_eq!(
        classify_harness("implement the new feature and ship it"),
        "loop-until-done"
    );
    assert_eq!(classify_harness(""), "loop-until-done");
}

#[test]
fn every_classification_is_a_known_pattern() {
    for prompt in [
        "triage x",
        "audit x",
        "review x",
        "brainstorm x",
        "compare x",
        "build x",
    ] {
        assert!(
            PATTERNS.contains(&classify_harness(prompt)),
            "classify_harness must only emit one of the six known patterns"
        );
    }
}

#[test]
fn persist_with_empty_project_is_noop_returning_block() {
    // Fail-soft: no project => no RPC, still returns the [HARNESS] context block
    // carrying the classified pattern.
    let block = persist_for_next_card("", "audit everything");
    assert!(block.contains("[HARNESS]"));
    assert!(block.contains("fan-out-synthesize"));
}

#[test]
fn parallel_patterns_directive_names_haiku_and_parallel_subagents() {
    // The fan-out (larger-implementation) pattern must tell the model to actually
    // spawn parallel subagents on the cheap Haiku tier — not just print jargon.
    let d = pattern_directive("fan-out-synthesize");
    assert!(d.contains("parallel"), "must direct parallel dispatch: {d}");
    assert!(d.contains(CHEAP_MODEL), "must name the Haiku tier: {d}");
    assert!(d.contains("Agent"), "must name the spawn mechanism: {d}");
}

#[test]
fn sequential_patterns_do_not_demand_parallel_fan_out() {
    // loop-until-done is single-threaded; its directive must not push parallel
    // subagents (would be wrong shape for the work).
    let d = pattern_directive("loop-until-done");
    assert!(
        !d.contains("parallel"),
        "sequential pattern is not fan-out: {d}"
    );
}

#[test]
fn every_pattern_has_a_nonempty_directive() {
    for p in PATTERNS {
        assert!(
            !pattern_directive(p).is_empty(),
            "pattern {p} must carry an actionable directive"
        );
    }
}

#[test]
fn persist_block_embeds_the_actionable_directive() {
    let block = persist_for_next_card("", "audit every handler across the workspace");
    assert!(block.contains("fan-out-synthesize"));
    assert!(
        block.contains(CHEAP_MODEL),
        "block must carry the directive: {block}"
    );
}
