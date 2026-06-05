//! Integration proof for the Layer-A bandit-log store (harness-rl Wave P2).
//!
//! The store is the durable substrate for the RLVR `(x, a, p, r)` tuple. These
//! tests prove against a real (in-memory) `SurrealDB` that a row persists and that
//! a replayed identical decision behaves as the store's doc claims — content
//! addressing must not silently double-count training signal.

use kavach_surreal::{append_bandit_row, open_memory};

const PAYLOAD: &str = r#"{"session_id":"sess_x","timestamp_ms":1717,"action":"block","propensity":1.0,"reward":null}"#;

#[tokio::test]
async fn append_persists_a_row_and_returns_its_content_addressed_id() {
    let db = open_memory().await.expect("memory db");
    let id = append_bandit_row(&db, PAYLOAD).await.expect("first append");

    // The id table is bandit_log and the key is the BLAKE3-derived digest.
    let id_str = format!("{id:?}");
    assert!(id_str.contains("bandit_log"), "id was {id_str}");

    // Prove the payload actually landed — read it back by the same key.
    let mut resp = db
        .query("SELECT VALUE payload.action FROM type::record('bandit_log', $k)")
        .bind(("k", blake3_key(PAYLOAD)))
        .await
        .expect("readback query");
    let action: Option<String> = resp.take(0).expect("take action");
    assert_eq!(action.as_deref(), Some("block"), "payload did not persist");
}

#[tokio::test]
async fn replaying_the_identical_decision_does_not_double_count() {
    // The doc contract: content addressing means one logical decision = one row.
    // SurrealDB CREATE on an existing id errors; the store must surface that
    // rather than silently writing a second row that would skew the OPE counts.
    let db = open_memory().await.expect("memory db");
    append_bandit_row(&db, PAYLOAD).await.expect("first append");
    let second = append_bandit_row(&db, PAYLOAD).await;

    // Whatever the outcome, the invariant that matters is: exactly one row.
    let mut resp = db
        .query("SELECT count() FROM bandit_log GROUP ALL")
        .await
        .expect("count query");
    let count: Option<i64> = resp.take((0, "count")).expect("take count");
    assert_eq!(count, Some(1), "identical replay must yield exactly one row");

    // And a rejected replay must be an Err, never a silent Ok masking the drop.
    assert!(second.is_err(), "a duplicate decision must not report success");
}

/// Mirror of the store's key derivation so the readback targets the same record.
fn blake3_key(payload: &str) -> String {
    let digest = blake3::hash(payload.as_bytes()).to_hex();
    digest.as_str().get(..32).unwrap_or(digest.as_str()).to_owned()
}
