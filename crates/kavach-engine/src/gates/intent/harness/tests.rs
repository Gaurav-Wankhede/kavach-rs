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
