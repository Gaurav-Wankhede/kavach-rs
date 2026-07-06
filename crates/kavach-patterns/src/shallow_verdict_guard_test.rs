//! Tests for the shallow-verdict guard.

use super::detect_shallow_verdict;

#[test]
fn shallow_clean_verdict_with_no_evidence_is_flagged() {
    // Exactly the failure mode from the session: "crates are clean" on a
    // cargo-tree + name-diff basis, no file:line.
    let msg = "All 20 crates are clean — cargo tree shows every crate reachable \
               and dead_code=deny proves no suppressed code.";
    assert!(detect_shallow_verdict(msg).is_some());
}

#[test]
fn wired_correctly_without_citation_is_flagged() {
    let msg = "The crates are wired correctly; every gate name in the dispatch \
               table is invoked by the config.";
    assert!(detect_shallow_verdict(msg).is_some());
}

#[test]
fn clean_verdict_with_file_line_is_allowed() {
    // Deep: cites the actual call-site body it read.
    let msg = "The decision path is clean: pre_tool.rs:24 dispatches \
               pre_tool_search::run on WebSearch, proven by reading the match arm.";
    assert!(detect_shallow_verdict(msg).is_none());
}

#[test]
fn verdict_with_rca_block_is_allowed() {
    let msg = "This is not a defect. [RCA] symptom: ...; root_cause: const literal \
               never reaches runtime; class: false_positive.";
    assert!(detect_shallow_verdict(msg).is_none());
}

#[test]
fn catches_correct_and_safe_verdicts_mandated_by_claudemd() {
    // ~/.claude/CLAUDE.md verdict_needs_leaf_evidence lists "correct" and "safe"
    // — both verdicts I made this session that the gate previously let pass.
    assert!(
        detect_shallow_verdict("the cast is safe; saturating to MAX never under-triggers")
            .is_some()
    );
    assert!(detect_shallow_verdict("this decode is correct by construction").is_some());
    assert!(detect_shallow_verdict("the producer/consumer contract is correct").is_some());
    // Still allowed when the verdict cites a leaf.
    assert!(
        detect_shallow_verdict("subagent.rs:122 saturation is fail-safe — proven at the call site")
            .is_none()
    );
}

#[test]
fn ordinary_message_without_verdict_is_ignored() {
    let msg = "I'll read the dispatch table next and trace the stop gate.";
    assert!(detect_shallow_verdict(msg).is_none());
}

#[test]
fn bare_verdict_without_citation_is_flagged() {
    // Red: bare clean verdict with no escape hatch.
    let msg = "the code is clean";
    assert!(detect_shallow_verdict(msg).is_some());
}

#[test]
fn verdict_with_file_line_citation_is_allowed() {
    // Green: file:line citation satisfies the guard.
    let msg = "clean — see foo.rs:42";
    assert!(detect_shallow_verdict(msg).is_none());
}

#[test]
fn verdict_with_uncertainty_qualifier_is_allowed() {
    // Green: explicit uncertainty qualifier ("not verified") is an honest hedge.
    let msg = "clean, but not verified";
    assert!(detect_shallow_verdict(msg).is_none());
    // Also test unverified variant.
    let msg2 = "the cast is safe, unverified";
    assert!(detect_shallow_verdict(msg2).is_none());
}

#[test]
fn file_line_detection_requires_a_digit_after_colon() {
    // "antiprod.rs:" with no line number is NOT a citation.
    let msg = "The file antiprod.rs: handles it and everything is clean.";
    assert!(detect_shallow_verdict(msg).is_some());
    // With a real line number it counts.
    let ok = "antiprod.rs:14 calls detect_antiprod; this path is clean.";
    assert!(detect_shallow_verdict(ok).is_none());
}
