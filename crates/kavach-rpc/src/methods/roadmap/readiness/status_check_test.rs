//! Tests for dispatch-eligibility predicates.
//! Owner-gating abolished (2026-06-20): there is no `is_gate` predicate — a
//! `GATE:` card is ordinary runnable work the agent claims and builds.
use super::{is_needs_decomposition, is_runnable_status, is_umbrella};

#[test]
fn todo_is_runnable() {
    assert!(is_runnable_status("todo"));
}

#[test]
fn verified_is_not_runnable() {
    assert!(!is_runnable_status("verified"));
}

#[test]
fn umbrella_title_detected() {
    assert!(is_umbrella("PLATFORM Health (NEW) — UMBRELLA, PLAN-FIRST"));
}

#[test]
fn plain_title_is_not_umbrella() {
    assert!(!is_umbrella("Port jobs browse page family"));
}

#[test]
fn lowercase_umbrella_prose_is_detected() {
    // The loop trap: "P4 JLM platform umbrella" (lowercase) slipped past the
    // case-sensitive check and re-dispatched forever. Must match now.
    assert!(is_umbrella(
        "P4 JLM platform umbrella (fundamentals verified)"
    ));
    assert!(is_umbrella("P3 Soundbak platform umbrella"));
}

#[test]
fn not_one_card_phrase_needs_decomposition() {
    assert!(is_needs_decomposition(
        "P7.F1 Dashboard internal shell — 127-PAGE PORT, NOT one-card auto-build."
    ));
}

#[test]
fn requiring_decomposition_phrase_detected() {
    assert!(is_needs_decomposition("MAJOR unit requiring decomposition"));
}

#[test]
fn decomposition_match_is_case_insensitive() {
    assert!(is_needs_decomposition("NOT ONE-CARD AUTO-BUILD"));
}

#[test]
fn atomic_card_does_not_need_decomposition() {
    assert!(!is_needs_decomposition(
        "Author POST /api/chat/messages/{id}/attachments confirm handler"
    ));
}

// PARKING ABOLISHED 2026-06-17: the `has_inert_blocker` tests were removed with
// the detector — the gate no longer recognizes OPERATOR-GATED:/AGENT_BLOCKED: prose.
// Dispatch eligibility is status + deps + umbrella only (decision.arch.harness-degate-stale-blocker).
