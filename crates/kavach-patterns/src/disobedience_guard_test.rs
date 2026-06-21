//! Red-Green proofs for the willful-disobedience detector. Outcomes drive the code.

use super::detect_disobedience;

#[test]
fn fires_on_loophole_lens_dismissed_as_na() {
    // The exact pattern from the transcript: loophole lens fired, dismissed in prose.
    let msg = "Loophole lens: N/A — comment-only edit, no logic changed.";
    assert!(detect_disobedience(msg).is_some(), "argue-not-obey must fire");
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
    assert!(detect_disobedience(msg).is_none(), "obey-proof clears the guard");
}

#[test]
fn clears_when_url_cited() {
    // Dismissal-ish word but a real source URL = research was done.
    let msg = "Loophole considered; per https://example.com/advisory it is N/A by design.";
    assert!(detect_disobedience(msg).is_none(), "a cited URL is obey-proof");
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
