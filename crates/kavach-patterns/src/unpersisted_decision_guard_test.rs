//! Tests for the unpersisted-decision advisory guard.

use super::detect_unpersisted_decision;

#[test]
fn settled_decision_in_prose_without_persistence_is_flagged() {
    let msg = "I'll use a single RPC daemon because the embedded store is single-writer.";
    assert!(detect_unpersisted_decision(msg, false).is_some());
}

#[test]
fn decision_persisted_via_db_write_is_not_flagged() {
    // The same prose, but the turn DID write to the DB.
    let msg = "I'll use a single RPC daemon. kavach db write --category decision ...";
    assert!(detect_unpersisted_decision(msg, false).is_none());
}

#[test]
fn decision_in_a_bracket_block_is_not_flagged() {
    // The bracket scanner already captures this — no double nudge.
    let msg = "[DESIGN] root cause: amfid inode cache; fix: fresh inode + exec-verify.";
    assert!(detect_unpersisted_decision(msg, false).is_none());
}

#[test]
fn db_write_witness_suppresses_the_nudge() {
    // Authoritative session witness: a decision row WAS written this turn.
    let msg = "The root cause is the inode cache; the fix is to copy fresh.";
    assert!(detect_unpersisted_decision(msg, true).is_none());
}

#[test]
fn ordinary_message_with_no_decision_is_ignored() {
    let msg = "I'll read the next file and trace the dispatch chain.";
    assert!(detect_unpersisted_decision(msg, false).is_none());
}

#[test]
fn a_question_is_not_a_settled_decision() {
    let msg = "Which approach should we use — a daemon or direct DB access?";
    assert!(detect_unpersisted_decision(msg, false).is_none());
}
