//! Red-Green proofs for the witness-receipt boundary token. Outcomes drive code.
//! SOURCE: decision.cli-verifier.witness-receipt-rpc-boundary.

use super::{Receipt, validate};

fn rcpt(passed: bool, git_head: &str, ts_ms: i64, session_id: &str) -> Receipt {
    Receipt {
        passed,
        git_head: git_head.to_owned(),
        ts_ms,
        session_id: session_id.to_owned(),
    }
}

#[test]
fn valid_fresh_matching_receipt_is_accepted() {
    let r = rcpt(true, "deadbeef", 1_000_000, "sess-1");
    // now == ts: zero age, within window; head matches; passed; session set.
    assert!(validate(&r, "deadbeef", 1_000_000, "sess-1").is_ok());
}

#[test]
fn failed_witness_is_refused() {
    let r = rcpt(false, "deadbeef", 1_000_000, "sess-1");
    assert!(validate(&r, "deadbeef", 1_000_000, "sess-1").is_err());
}

#[test]
fn stale_receipt_beyond_window_is_refused() {
    let r = rcpt(true, "deadbeef", 1_000_000, "sess-1");
    // 5min + 1ms later — past the freshness window.
    assert!(validate(&r, "deadbeef", 1_000_000 + 300_001, "sess-1").is_err());
}

#[test]
fn head_mismatch_is_refused() {
    // The tree moved since the witness ran — the receipt no longer proves THIS code.
    let r = rcpt(true, "oldsha", 1_000_000, "sess-1");
    assert!(validate(&r, "newsha", 1_000_000, "sess-1").is_err());
}

#[test]
fn empty_session_is_refused() {
    let r = rcpt(true, "deadbeef", 1_000_000, "");
    assert!(validate(&r, "deadbeef", 1_000_000, "").is_err());
}

#[test]
fn cross_session_replay_is_refused() {
    // A receipt minted by another session cannot promote this session's card.
    let r = rcpt(true, "deadbeef", 1_000_000, "sess-OTHER");
    assert!(validate(&r, "deadbeef", 1_000_000, "sess-1").is_err());
}

#[test]
fn future_dated_receipt_is_refused() {
    // ts in the future (clock skew / forgery) — refuse rather than accept blindly.
    let r = rcpt(true, "deadbeef", 2_000_000, "sess-1");
    assert!(validate(&r, "deadbeef", 1_000_000, "sess-1").is_err());
}

#[test]
fn empty_git_head_allowed_when_daemon_also_has_no_head() {
    // Non-git projects have no HEAD; both sides read "". Session+timestamp
    // binding still applies.
    let r = rcpt(true, "", 1_000_000, "sess-1");
    assert!(validate(&r, "", 1_000_000, "sess-1").is_ok());
}

#[test]
fn empty_git_head_refused_when_daemon_has_a_head() {
    // A receipt minted outside a git repo must not promote a git-backed project.
    let r = rcpt(true, "", 1_000_000, "sess-1");
    assert!(validate(&r, "deadbeef", 1_000_000, "sess-1").is_err());
}
