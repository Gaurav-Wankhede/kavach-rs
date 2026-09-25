use super::{raw_select, strip_comments};
use crate::open_memory;

#[tokio::test]
async fn select_returns_json() {
    let db = open_memory().await.expect("mem db");
    db.exec_params(
        "INSERT INTO entities (id, entity_type, name) VALUES ('entity:a', 'x', 'a')",
        (),
    )
    .await
    .expect("seed");
    let out = raw_select(&db, "SELECT name FROM entities")
        .await
        .expect("select runs");
    assert!(out.contains("\"name\""), "json has the column: {out}");
    assert!(out.contains('a'), "json has the value: {out}");
}

#[tokio::test]
async fn missing_table_is_empty_array() {
    let db = open_memory().await.expect("mem db");
    let out = raw_select(&db, "SELECT * FROM never_created")
        .await
        .expect("empty, not error");
    assert_eq!(out.trim(), "[]", "fresh table -> empty: {out}");
}

#[tokio::test]
async fn delete_is_refused() {
    let db = open_memory().await.expect("mem db");
    let err = raw_select(&db, "DELETE FROM entities")
        .await
        .expect_err("write rejected");
    assert!(err.to_string().contains("read-only"), "got: {err}");
}

#[tokio::test]
async fn smuggled_write_after_select_is_refused() {
    let db = open_memory().await.expect("mem db");
    // The second statement must be caught even though the first is a valid SELECT.
    let err = raw_select(&db, "SELECT 1; UPDATE entities SET name = 'x'")
        .await
        .expect_err("multi-statement write rejected");
    assert!(err.to_string().contains("read-only"), "got: {err}");
}

#[tokio::test]
async fn comment_hidden_write_is_refused() {
    let db = open_memory().await.expect("mem db");
    // A block comment must not let a write verb masquerade as the leading token.
    let err = raw_select(&db, "/* SELECT */ DELETE FROM entities")
        .await
        .expect_err("comment-masked write rejected");
    assert!(err.to_string().contains("read-only"), "got: {err}");
}

#[tokio::test]
async fn empty_query_is_refused() {
    let db = open_memory().await.expect("mem db");
    let err = raw_select(&db, "   -- just a comment\n  ")
        .await
        .expect_err("empty rejected");
    assert!(err.to_string().contains("empty"), "got: {err}");
}

#[test]
fn strip_comments_removes_line_and_block() {
    let s = strip_comments("SELECT 1 -- tail\n/* mid */ FROM x # hash\n");
    assert!(!s.contains("tail"), "line comment gone: {s}");
    assert!(!s.contains("mid"), "block comment gone: {s}");
    assert!(!s.contains("hash"), "hash comment gone: {s}");
    assert!(s.contains("SELECT 1"), "code kept: {s}");
    assert!(s.contains("FROM x"), "code kept: {s}");
}

#[tokio::test]
async fn select_star_over_blob_columns_does_not_panic() {
    // K-2: `SELECT *` over rows carrying BLOB columns aborted the CLI with
    // `entered unreachable code: invalid value type` (exit 101) because the
    // old typed `get::<String/i64/f64>` chain cannot decode blobs. A dedicated
    // BLOB probe table proves the untyped path renders them as markers, and a
    // star-select over `memories` (home of `embedding F32_BLOB`) runs clean.
    let db = open_memory().await.expect("mem db");
    db.exec_params("CREATE TABLE blob_probe (name TEXT, data BLOB)", ())
        .await
        .expect("probe table");
    db.exec_params(
        "INSERT INTO blob_probe (name, data) VALUES ('v', randomblob(32))",
        (),
    )
    .await
    .expect("blob seed");
    let out = raw_select(&db, "SELECT * FROM blob_probe")
        .await
        .expect("star select over blob runs without panic");
    assert!(out.contains("blob:"), "blob renders as marker: {out}");
    db.exec_params(
        "INSERT INTO memories (project, category, entry_key, title, content, created_at, updated_at) VALUES ('p', 'roadmap', 'k', 't', 'c', datetime('now'), datetime('now'))",
        (),
    )
    .await
    .expect("seed");
    let mem = raw_select(&db, "SELECT * FROM memories")
        .await
        .expect("memories star select runs");
    assert!(mem.contains("\"embedding\""), "all columns present: {mem}");
}

#[tokio::test]
async fn group_by_status_runs() {
    // K-2 second repro shape: aggregation over the store must not abort.
    let db = open_memory().await.expect("mem db");
    let out = raw_select(
        &db,
        "SELECT status, count(*) AS n FROM memories GROUP BY status",
    )
    .await
    .expect("group-by runs");
    assert!(out.contains('[') || out.contains('{'), "json shape: {out}");
}

#[tokio::test]
async fn pragma_table_info_is_allowed() {
    // K-3: read-only schema inspectors are admitted.
    let db = open_memory().await.expect("mem db");
    let out = raw_select(&db, "PRAGMA table_info(memories)")
        .await
        .expect("table_info runs");
    assert!(out.contains("entry_key"), "schema lists columns: {out}");
}

#[tokio::test]
async fn pragma_table_list_is_allowed() {
    let db = open_memory().await.expect("mem db");
    let out = raw_select(&db, "PRAGMA table_list")
        .await
        .expect("table_list runs");
    assert!(out.contains("memories"), "schema lists tables: {out}");
}

#[tokio::test]
async fn pragma_journal_mode_stays_rejected() {
    // Only the schema inspectors are allowlisted — PRAGMAs that mutate state
    // (journal_mode, synchronous, wal_checkpoint) stay refused.
    let db = open_memory().await.expect("mem db");
    let err = raw_select(&db, "PRAGMA journal_mode=WAL")
        .await
        .expect_err("state-changing pragma rejected");
    assert!(err.to_string().contains("read-only"), "got: {err}");
}
