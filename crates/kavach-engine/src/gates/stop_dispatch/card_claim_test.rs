//! `claim_card` fail-closed contract: the guard clauses return `false` for any
//! input that cannot name a real row, WITHOUT issuing an RPC. Proves the safe
//! direction for an at-most-once claim — an unnameable target is never "won here".
//! See decision.engine.claim-card-fail-closed.

use super::SOURCE_DOWN_KEY;
use super::claim_card;

#[test]
fn empty_project_never_claims() {
    assert!(
        !claim_card("", "roadmap.real-key"),
        "empty project must fail closed"
    );
}

#[test]
fn empty_key_never_claims() {
    assert!(!claim_card("proj", ""), "empty key must fail closed");
}

#[test]
fn source_down_sentinel_never_claims() {
    // The RPC-outage sentinel is not a real row — claiming it would forge a win.
    assert!(
        !claim_card("proj", SOURCE_DOWN_KEY),
        "the source-down sentinel must never be claimed"
    );
}
