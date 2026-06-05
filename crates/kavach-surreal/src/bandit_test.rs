//! Pure-helper proofs for the `bandit_log` store. The DB-backed paths are covered
//! by `tests/bandit_log.rs`; here we prove the content-addressing + the two
//! Rust-side filters that decide which rows the reward back-fill grades.

use super::{content_key, pending_for_session, reward_is_absent};

#[test]
fn content_key_is_deterministic_and_32_hex() {
    let k1 = content_key(r#"{"a":1}"#);
    let k2 = content_key(r#"{"a":1}"#);
    assert_eq!(k1, k2, "same payload ⇒ same key (idempotent append/update)");
    assert_eq!(k1.len(), 32, "first 32 hex of the BLAKE3 digest");
    assert!(k1.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn content_key_differs_for_different_payloads() {
    assert_ne!(content_key(r#"{"a":1}"#), content_key(r#"{"a":2}"#));
}

#[test]
fn reward_is_absent_for_null_or_missing_only() {
    assert!(
        reward_is_absent(r#"{"action":"allow"}"#),
        "missing reward ⇒ pending"
    );
    assert!(
        reward_is_absent(r#"{"reward":null}"#),
        "JSON null reward ⇒ pending"
    );
    assert!(
        reward_is_absent("not json"),
        "unparseable ⇒ surfaced as pending"
    );
    assert!(
        !reward_is_absent(r#"{"reward":"verified_clean"}"#),
        "graded ⇒ NOT pending"
    );
    assert!(
        !reward_is_absent(r#"{"reward":"false_decision"}"#),
        "graded ⇒ NOT pending"
    );
}

#[test]
fn pending_for_session_matches_only_an_unrewarded_row_of_the_join_key() {
    // The JOIN predicate: un-rewarded AND this session. Reward absent on all
    // rows here so the session axis is what's under test.
    let row = r#"{"session_id":"sess_abc","action":"block","reward":null}"#;
    assert!(pending_for_session(row, "sess_abc"));
    assert!(
        !pending_for_session(row, "sess_other"),
        "another session's row ⇒ not in this join"
    );
    // A graded row of THIS session is also excluded — the reward axis still bites.
    let graded = r#"{"session_id":"sess_abc","action":"block","reward":"needed_ask"}"#;
    assert!(
        !pending_for_session(graded, "sess_abc"),
        "already graded ⇒ not a candidate"
    );
}

#[test]
fn a_row_missing_its_session_id_does_not_match_a_real_session() {
    let row = r#"{"action":"block","reward":null}"#;
    assert!(
        !pending_for_session(row, "sess_abc"),
        "no session_id field ⇒ not this session"
    );
}

#[test]
fn an_unparseable_row_is_kept_so_it_surfaces() {
    // Parse failure ⇒ treated as a match, so a malformed row is surfaced to the
    // caller rather than silently dropped from the grading set.
    assert!(pending_for_session("not json", "sess_abc"));
}
