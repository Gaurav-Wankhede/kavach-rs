//! Tests for owner-gate-shape detection + word-stripping + ACT imperative.
use super::*;

#[test]
fn detects_gate_prefix_and_markers() {
    assert!(is_gate_shaped("GATE: deploy cf worker to prod"));
    assert!(is_gate_shaped("OWNER-GATE: provision the DO namespace"));
    assert!(is_gate_shaped("CLASS-B owner gate — live-payment cohort"));
    assert!(is_gate_shaped("await greenlight on WDR live-prayer"));
    assert!(is_gate_shaped("decision.owner-action.cf-deploy-removed"));
}

#[test]
fn does_not_flag_ordinary_cards() {
    // A normal buildable card must stay on the plain dispatch path — no
    // false-positive that would strip a real title.
    assert!(!is_gate_shaped("Implement the rate-limiter middleware"));
    assert!(!is_gate_shaped("Refactor upsert_entry_full to params struct"));
    assert!(!is_gate_shaped("Fix supersedes projection silent failure"));
}

#[test]
fn strips_gate_prefix_leaving_the_work() {
    assert_eq!(
        strip_gate_words("GATE: wire DO WS backlog replay"),
        "wire DO WS backlog replay"
    );
    assert_eq!(
        strip_gate_words("OWNER-GATE: ship migration 276"),
        "ship migration 276"
    );
}

#[test]
fn strips_inline_markers_and_collapses_separators() {
    let out = strip_gate_words("CLASS-B owner gate — live-payment cohort");
    assert!(!out.to_lowercase().contains("class-b"));
    assert!(!out.to_lowercase().contains("owner gate"));
    assert!(out.contains("live-payment cohort"));
}

#[test]
fn strip_never_returns_empty() {
    // A title that is ONLY gate words must fall back to the original, never a
    // blank card handed to the agent.
    let out = strip_gate_words("GATE:");
    assert!(!out.is_empty());
}

#[test]
fn act_imperative_forbids_holding_and_names_the_work() {
    let d = act_imperative("ship migration 276");
    assert!(d.contains("ship migration 276"), "names the stripped work");
    assert!(d.contains("DO NOT HOLD"), "imperative register");
    let lower = d.to_lowercase();
    // The whole point: the directive must forbid the looped hold + hand-back.
    assert!(lower.contains("holding") && lower.contains("forbidden"));
    // And it must offer the two runnable exits (split / delete) that close the
    // card instead of re-dispatching it.
    assert!(lower.contains("delete") && lower.contains("split"));
}
