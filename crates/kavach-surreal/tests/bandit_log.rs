//! Integration proof for the Layer-A bandit-log store (harness-rl Wave P2).
//!
//! The store is the durable substrate for the RLVR `(x, a, p, r)` tuple. These
//! tests prove against a real (in-memory) `SurrealDB` that a row persists and that
//! a replayed identical decision behaves as the store's doc claims — content
//! addressing must not silently double-count training signal.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test assertions: a panic on the Err/None path IS the failure signal"
)]

use kavach_surreal::{
    append_bandit_row, apply_schema, list_bandit_rows, list_unrewarded_bandit_rows, open_memory,
    update_bandit_reward,
};

/// Open an in-memory db with the production schema applied — the daemon path
/// (`open_default_daemon`) runs `apply_schema`, so a read of a never-written
/// `bandit_log` must see a *defined, empty* table, not `SurrealDB` 3.0's
/// "table does not exist" error. `open_memory` alone skips schema.
async fn open_with_schema() -> surrealdb::Surreal<surrealdb::engine::local::Db> {
    let db = open_memory().await.expect("memory db");
    apply_schema(&db).await.expect("apply schema");
    db
}

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

#[tokio::test]
async fn list_bandit_rows_reads_back_appended_payloads() {
    // The OPE layer (kavach-ope) consumes exactly these payloads; prove the
    // read path returns them intact and as valid JSON the estimators can parse.
    let db = open_with_schema().await;
    let a = r#"{"session_id":"s1","timestamp_ms":1,"action":"allow","propensity":1.0,"reward":1}"#;
    let b = r#"{"session_id":"s2","timestamp_ms":2,"action":"block","propensity":1.0,"reward":null}"#;
    append_bandit_row(&db, a).await.expect("append a");
    append_bandit_row(&db, b).await.expect("append b");

    let rows = list_bandit_rows(&db, 100).await.expect("list");
    assert_eq!(rows.len(), 2, "both appended rows must come back");

    // Each returned string parses as JSON and carries the action field.
    let actions: Vec<String> = rows
        .iter()
        .map(|r| {
            let v: serde_json::Value = serde_json::from_str(r).expect("payload is valid JSON");
            v["action"].as_str().expect("action field").to_owned()
        })
        .collect();
    assert!(actions.contains(&"allow".to_owned()), "got {actions:?}");
    assert!(actions.contains(&"block".to_owned()), "got {actions:?}");
}

#[tokio::test]
async fn list_bandit_rows_is_empty_on_a_fresh_db() {
    // With the schema applied, `bandit_log` is a DEFINED but empty table — the
    // read must return [], not `SurrealDB` 3.0's "table does not exist" error.
    let db = open_with_schema().await;
    let rows = list_bandit_rows(&db, 100).await.expect("list");
    assert!(rows.is_empty(), "no rows logged yet");
}

#[tokio::test]
async fn unrewarded_list_excludes_rows_whose_reward_is_set() {
    // P3a: only rows still awaiting a 3-witness reward are back-fill candidates.
    let db = open_with_schema().await;
    let pending = r#"{"session_id":"p","timestamp_ms":1,"action":"allow","propensity":1.0,"reward":null}"#;
    let graded = r#"{"session_id":"g","timestamp_ms":2,"action":"block","propensity":1.0,"reward":"needed_ask"}"#;
    append_bandit_row(&db, pending).await.expect("append pending");
    append_bandit_row(&db, graded).await.expect("append graded");

    let unrewarded = list_unrewarded_bandit_rows(&db, 100).await.expect("list unrewarded");
    assert_eq!(unrewarded.len(), 1, "only the null-reward row is a candidate");
    let v: serde_json::Value = serde_json::from_str(&unrewarded[0]).expect("json");
    assert_eq!(v["session_id"].as_str(), Some("p"));
    assert!(v["reward"].is_null(), "candidate still awaits its reward");
}

#[tokio::test]
async fn back_filling_a_reward_removes_the_row_from_the_unrewarded_list() {
    // P3a write path: update_bandit_reward sets the reward by re-deriving the
    // content-addressed key from the SAME payload — after which the row is no
    // longer a back-fill candidate, and the reward reads back on the row.
    let db = open_with_schema().await;
    let payload = r#"{"session_id":"x","timestamp_ms":7,"action":"block","propensity":1.0,"reward":null}"#;
    append_bandit_row(&db, payload).await.expect("append");
    assert_eq!(list_unrewarded_bandit_rows(&db, 100).await.expect("pre").len(), 1);

    update_bandit_reward(&db, payload, "false_decision").await.expect("back-fill");

    assert!(
        list_unrewarded_bandit_rows(&db, 100).await.expect("post").is_empty(),
        "a back-filled row is no longer un-rewarded"
    );
    // And the reward actually landed on the stored row.
    let all = list_bandit_rows(&db, 100).await.expect("all");
    let v: serde_json::Value = serde_json::from_str(&all[0]).expect("json");
    assert_eq!(v["reward"].as_str(), Some("false_decision"), "reward persisted");
}

#[tokio::test]
async fn back_filling_an_absent_row_is_an_error_not_a_silent_noop() {
    // Fail-closed: an UPDATE matching no row must surface, never report success.
    let db = open_with_schema().await;
    let never_logged = r#"{"session_id":"ghost","timestamp_ms":9,"action":"allow","propensity":1.0,"reward":null}"#;
    let res = update_bandit_reward(&db, never_logged, "verified_clean").await;
    assert!(res.is_err(), "back-filling a row that was never logged must error");
}
