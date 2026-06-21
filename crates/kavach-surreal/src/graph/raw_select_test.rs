use super::{raw_select, strip_comments};
use crate::open_memory;

#[tokio::test]
async fn select_returns_json() {
    let db = open_memory().await.expect("mem db");
    db.query("CREATE entity:a SET entity_type = 'x', n = 1")
        .await
        .expect("seed");
    let out = raw_select(&db, "SELECT n FROM entity")
        .await
        .expect("select runs");
    assert!(out.contains("\"n\""), "json has the column: {out}");
    assert!(out.contains('1'), "json has the value: {out}");
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
    let err = raw_select(&db, "DELETE entity")
        .await
        .expect_err("write rejected");
    assert!(err.to_string().contains("read-only"), "got: {err}");
}

#[tokio::test]
async fn smuggled_write_after_select_is_refused() {
    let db = open_memory().await.expect("mem db");
    // The second statement must be caught even though the first is a valid SELECT.
    let err = raw_select(&db, "SELECT 1; UPDATE entity SET x = 9")
        .await
        .expect_err("multi-statement write rejected");
    assert!(err.to_string().contains("read-only"), "got: {err}");
}

#[tokio::test]
async fn comment_hidden_write_is_refused() {
    let db = open_memory().await.expect("mem db");
    // A block comment must not let a write verb masquerade as the leading token.
    let err = raw_select(&db, "/* SELECT */ DELETE entity")
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
