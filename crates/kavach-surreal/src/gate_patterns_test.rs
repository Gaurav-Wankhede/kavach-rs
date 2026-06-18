// Proves the loop-engineering Frame-1 changes to `list_hot`: the SELECT now
// (a) ORDER BY occurrence_count DESC (was unordered LIMIT — the ExpeL flaw) and
// (b) surfaces `updated_unix` (time::unix(updated_at)) so the SessionStart
// consumer can rank by k_pri recency. The recency×recurrence scoring itself is
// proven in k_pri's own tests; here we prove the SQL feeds it correctly.
use super::list_hot;
use crate::open_memory;
use surrealdb_types::RecordId;

/// CREATE one autonomous-tier `gate_pattern` with an explicit `occurrence_count`.
async fn seed(
    db: &surrealdb::Surreal<surrealdb::engine::any::Any>,
    proj: &RecordId,
    tokens: &str,
    count: i64,
) {
    db.query(
        "CREATE gate_pattern SET project = $p, tool_name = 't', gate_name = 'g', \
         error_tokens = $e, fix_strategy = 'f', imperative_rewrite = '', \
         dsa_rationale = '', occurrence_count = $c, tier = 'autonomous', \
         updated_at = time::now()",
    )
    .bind(("p", proj.clone()))
    .bind(("e", tokens.to_owned()))
    .bind(("c", count))
    .await
    .expect("seed autonomous gate_pattern");
}

#[tokio::test]
async fn list_hot_orders_by_occurrence_desc_and_surfaces_updated_unix() {
    let db = open_memory().await.expect("open in-memory db");
    let proj = RecordId::new("project", "test-proj");
    seed(&db, &proj, "low", 5).await;
    seed(&db, &proj, "high", 50).await;
    seed(&db, &proj, "mid", 10).await;

    let rows = list_hot(&db, &proj, 10).await.expect("list_hot");
    assert_eq!(rows.len(), 3, "all 3 autonomous rows returned, got {rows:?}");
    // ORDER BY occurrence_count DESC — was unordered before Frame 1.
    assert_eq!(rows[0].occurrence_count, 50, "highest occurrence ranks first");
    assert_eq!(rows[1].occurrence_count, 10);
    assert_eq!(rows[2].occurrence_count, 5);
    // updated_unix surfaced from time::unix(updated_at) for the recency axis.
    assert!(
        rows[0].updated_unix.is_some(),
        "updated_unix must surface so the consumer can rank by k_pri recency"
    );
}

#[tokio::test]
async fn list_hot_is_empty_not_error_when_table_absent() {
    // A fresh DB has never recorded a gate_pattern, so the table does not exist.
    // list_hot must fail closed to an empty list, never propagate a SELECT error
    // (the SessionStart consumer would otherwise lose its injection on a benign
    // empty state).
    let db = open_memory().await.expect("open in-memory db");
    let proj = RecordId::new("project", "test-proj");
    let rows = list_hot(&db, &proj, 10)
        .await
        .expect("missing table must be the empty case, not an error");
    assert!(rows.is_empty(), "no patterns ⇒ empty, got {rows:?}");
}

#[tokio::test]
async fn list_hot_excludes_non_autonomous_tier() {
    let db = open_memory().await.expect("open in-memory db");
    let proj = RecordId::new("project", "test-proj");
    db.query(
        "CREATE gate_pattern SET project = $p, tool_name = 't', gate_name = 'g', \
         error_tokens = 'x', fix_strategy = 'f', imperative_rewrite = '', \
         dsa_rationale = '', occurrence_count = 99, tier = 'research', \
         updated_at = time::now()",
    )
    .bind(("p", proj.clone()))
    .await
    .expect("seed research-tier row");

    let rows = list_hot(&db, &proj, 10).await.expect("list_hot");
    assert!(rows.is_empty(), "research-tier rows are not hot, got {rows:?}");
}
