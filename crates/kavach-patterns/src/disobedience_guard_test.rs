//! Red-Green proofs for the willful-disobedience detector. Outcomes drive the code.

use super::detect_disobedience;

#[test]
fn fires_on_loophole_lens_dismissed_as_na() {
    // The exact pattern from the transcript: loophole lens fired, dismissed in prose.
    let msg = "Loophole lens: N/A — comment-only edit, no logic changed.";
    assert!(
        detect_disobedience(msg).is_some(),
        "argue-not-obey must fire"
    );
}

#[test]
fn fires_on_research_imperative_dismissed() {
    let msg = "The research-first advisory doesn't apply here, no need to WebSearch.";
    assert!(detect_disobedience(msg).is_some());
}

#[test]
fn clears_when_loopholes_actually_closed_with_proof() {
    // Obeyed: named the lens AND closed it with a file:line marker.
    let msg = "Loopholes closed: concurrency FIXED at stop.rs:120 via compare-and-swap.";
    assert!(
        detect_disobedience(msg).is_none(),
        "obey-proof clears the guard"
    );
}

#[test]
fn clears_when_url_cited() {
    // Dismissal-ish word but a real source URL = research was done.
    let msg = "Loophole considered; per https://example.com/advisory it is N/A by design.";
    assert!(
        detect_disobedience(msg).is_none(),
        "a cited URL is obey-proof"
    );
}

#[test]
fn no_marker_no_fire() {
    // Dismissal vocab but NO imperative marker = ordinary prose, must not fire.
    let msg = "This field is N/A for free accounts.";
    assert!(detect_disobedience(msg).is_none());
}

#[test]
fn clean_completion_does_not_fire() {
    let msg = "Built the feature, 933 tests pass, diff landed.";
    assert!(detect_disobedience(msg).is_none());
}

#[test]
fn fires_on_self_verify_dodge_of_agent_spawn() {
    let msg = "The agent-spawn advisory fired but I'll just verify myself instead.";
    assert!(detect_disobedience(msg).is_some());
}

#[test]
fn default_vocab_matches_floor_detector() {
    use super::{DisobedienceVocab, detect_disobedience_with};
    // The floor-default vocab must reproduce the compiled detector exactly.
    let v = DisobedienceVocab::default();
    let msg = "Loophole lens: N/A — comment-only edit.";
    assert_eq!(
        detect_disobedience_with(&v, msg).is_some(),
        detect_disobedience(msg).is_some()
    );
}

#[test]
fn graph_overlay_adds_dismissal_phrase_floor_still_active() {
    use super::{DisobedienceVocab, detect_disobedience_with};
    // ADDITIVE override: a project adds a new dismissal phrase; the floor markers
    // (loophole) + obey-proof still apply. Graph ADDS, never replaces the floor.
    let mut v = DisobedienceVocab::default();
    v.dismissal.push("punting on this".to_owned());
    let msg = "The loophole advisory? punting on this for now.";
    assert!(
        detect_disobedience_with(&v, msg).is_some(),
        "added phrase fires"
    );
    // floor obey-proof still clears it
    let cleared = "loophole punting on this — but Loopholes closed: x at a.rs:1.";
    assert!(
        detect_disobedience_with(&v, cleared).is_none(),
        "floor obey-proof intact"
    );
}

#[test]
fn malformed_overlay_degrades_to_floor() {
    use super::DisobedienceVocab;
    // serde(default): an empty JSON object yields the full compiled floor.
    let v: DisobedienceVocab = serde_json::from_str("{}").expect("empty obj is valid");
    assert!(!v.dismissal.is_empty() && !v.imperative_marker.is_empty() && !v.obeyed.is_empty());
}
