use super::*;

#[test]
fn build_block_formats() {
    let msg = build_block("TEST_GUARD", &[("P0_CODE", "reason text")]);
    assert!(msg.contains("[TEST_GUARD_PLATFORM_POLICY]"));
    assert!(msg.contains("P0_CODE"));
    assert!(msg.contains("reason text"));
    assert!(msg.contains("retry"));
}

#[test]
fn build_block_multiple_violations() {
    let msg = build_block("X", &[("A", "reason a"), ("B", "reason b")]);
    assert!(msg.contains("A — reason a"));
    assert!(msg.contains("B — reason b"));
}

#[test]
fn build_advisory_formats() {
    let msg = build_advisory("TEST_GUARD", &[("P1_CODE", "advisory text")]);
    assert!(msg.contains("[TEST_GUARD_ADVISORY]"));
    assert!(msg.contains("P1_CODE"));
    assert!(msg.contains("advisory text"));
}

#[test]
fn build_advisory_empty_violations() {
    let msg = build_advisory("X", &[]);
    assert!(msg.contains("[X_ADVISORY]"));
}
