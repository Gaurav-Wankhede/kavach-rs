use super::lane_drained_context;

#[test]
fn lane_drained_context_is_a_clean_stop_naming_the_lane() {
    // A laned session whose lane + unlaned are drained must clean-stop, name
    // the lane, and refuse to dispatch foreign-lane cards or invent plans.
    let c = lane_drained_context("crypto");
    assert!(c.contains("LANE_DRAINED"), "tag present: {c}");
    assert!(c.contains("crypto"), "names the drained lane: {c}");
    assert!(
        c.contains("FOREIGN lane"),
        "warns against crossing into foreign work: {c}"
    );
    assert!(c.contains("Clean stop"), "is a clean stop: {c}");
    assert!(!c.contains("AUTO_CONTINUE"), "must NOT spin the loop: {c}");
}
