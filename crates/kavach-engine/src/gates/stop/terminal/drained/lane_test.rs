use super::lane_drained_context;

#[test]
fn lane_drained_context_rescans_the_lane_without_stop_language() {
    // A laned session whose lane + unlaned are drained re-scans its OWN lane's
    // DB rows, names the lane, refuses to cross into foreign work, and never
    // self-terminates — it yields only to the user's Esc.
    let c = lane_drained_context("crypto");
    assert!(c.contains("LANE_DRAINED"), "tag present: {c}");
    assert!(c.contains("crypto"), "names the drained lane: {c}");
    assert!(
        c.contains("FOREIGN lane"),
        "warns against crossing into foreign work: {c}"
    );
    assert!(c.contains("Do NOT stop"), "never self-terminates: {c}");
    assert!(c.contains("Esc"), "yields only to the user halt: {c}");
    assert!(
        !c.contains("Clean stop") && !c.contains("clean stop"),
        "carries no clean-stop language: {c}"
    );
}
