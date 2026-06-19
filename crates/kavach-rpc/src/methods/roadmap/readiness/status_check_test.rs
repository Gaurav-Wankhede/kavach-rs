//! Tests for dispatch-eligibility predicates.
use super::{is_gate, is_needs_decomposition, is_runnable_status, is_umbrella};

/// A GATE: card is an operator-decision node — never agent-dispatched.
#[test]
fn gate_titled_card_is_a_gate() {
    assert!(is_gate(
        "GATE: operator opens live-Neon maintenance window + supplies DATABASE_URL"
    ));
}

/// Gate detection is case-insensitive and tolerates leading whitespace.
#[test]
fn gate_detection_is_case_and_whitespace_insensitive() {
    assert!(is_gate("  gate: operator greenlights money paths"));
}

/// A normal buildable card whose title merely mentions "gate" is NOT a gate
/// (the marker is the `GATE:` PREFIX, not the substring — avoids false-excluding
/// e.g. a card about the dispatch gate itself).
#[test]
fn substring_gate_is_not_a_gate_card() {
    assert!(!is_gate("Fix the micro-file split gate blind to Edit"));
    assert!(!is_gate("Author POST /api/... handler"));
}

/// The `GATE (operator): …` parenthetical form is the ACTUAL card convention — it
/// MUST be recognized. The prior `starts_with("gate:")` missed it, leaking every
/// operator gate into dispatch and re-looping forever (predicate-drift fix 2026-06-19).
#[test]
fn gate_operator_parenthetical_form_is_a_gate() {
    assert!(is_gate(
        "GATE (operator): money-path greenlight — authorize live Stripe/PayPal settlement"
    ));
}

/// A near-miss: a title beginning with the WORD "GATES" (no `:` convention) is
/// NOT a gate card — the matcher keys on `GATE` + qualifier + `:`, not the substring.
#[test]
fn gates_word_prefix_is_not_a_gate_card() {
    assert!(!is_gate("GATES to BDF cart-checkout delegated downstream"));
}

/// A prose title starting "Gateway …" must not false-positive (alnum after GATE).
#[test]
fn gateway_prefix_is_not_a_gate_card() {
    assert!(!is_gate("Gateway circuit-breaker tuning for the action layer"));
}

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
    assert!(is_umbrella("P4 JLM platform umbrella (fundamentals verified)"));
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
