use super::dispatch_msg::dispatch_message;

#[test]
fn frames_as_retestable_condition_not_standing_block() {
    let m = dispatch_message(1, 3, "TIER0", "roadmap.x", "Build X", "");
    assert!(
        m.contains("RUNNABLE NOW"),
        "dispatch must label state as runnable, got: {m}"
    );
    assert!(
        m.contains("re-testable condition"),
        "dispatch must frame the gate as a re-testable condition, got: {m}"
    );
    assert!(
        !m.contains("STOP BLOCKED"),
        "stale standing-block label must be gone, got: {m}"
    );
}

#[test]
fn preserves_dispatch_contract_fields() {
    let m = dispatch_message(2, 3, "TIER1", "roadmap.y", "Title Y", "[HARNESS]");
    assert!(m.contains("(2/3)"), "attempt/max preserved");
    assert!(m.contains("TIER1 [roadmap.y]: Title Y"), "next-card line preserved");
    assert!(m.contains("FAN IT OUT NOW"), "fan-out directive preserved");
    assert!(m.ends_with("[HARNESS]"), "harness suffix preserved");
}
