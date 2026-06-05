use super::*;

fn empty_session() -> SessionState {
    SessionState::default()
}

fn verified_session() -> SessionState {
    let mut s = SessionState::default();
    s.recent_commands.push("cargo test --workspace".into());
    s
}

#[test]
fn no_claim_passes() {
    let s = empty_session();
    assert!(check_completion_claim("fn main() {}", &s).is_none());
}

#[test]
fn claim_without_evidence_warns() {
    let s = empty_session();
    let r = check_completion_claim("All tests pass!", &s);
    assert!(r.is_some());
    assert!(r.unwrap().contains("COMPLETION_CHECK"));
}

#[test]
fn claim_with_evidence_passes() {
    let s = verified_session();
    let r = check_completion_claim("All tests pass!", &s);
    assert!(r.is_none());
}

// NOTE: the review-isolation tests were removed alongside `check_review_isolation`
// under the "kill blocking, keep auto-continue" policy (the Stop gate no longer
// halts on completion-language + modified-file count).
