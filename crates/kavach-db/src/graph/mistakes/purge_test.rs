// Delete-side proofs: empty-gate guard, missing-table no-op, and a full
// seed→purge→verify-gone roundtrip mirroring top_test's capture path.
use super::delete_anti_patterns_by_gate;
use crate::error::Result;
use crate::graph::mistakes::{append_mistake_event, cluster_event_to_pattern, top_anti_patterns};
use crate::open_memory;

/// Route one mistake observation through the capture path (same as `top_test::seed`).
async fn seed(
    db: &surrealdb::Surreal<surrealdb::engine::any::Any>,
    gate: &str,
    fix: &str,
) -> Result<()> {
    let ev = append_mistake_event(db, gate, fix, "banned phrase", "sess", Some("proj"), 0).await?;
    cluster_event_to_pattern(db, &ev, gate, fix).await?;
    Ok(())
}

#[tokio::test]
async fn empty_gate_deletes_nothing() {
    let db = open_memory().await.expect("open in-memory db");
    let n = delete_anti_patterns_by_gate(&db, "")
        .await
        .expect("empty gate is a clean no-op");
    assert_eq!(n, 0, "an empty gate must purge nothing");
}

#[tokio::test]
async fn missing_table_is_the_empty_case() {
    // Fresh DB never created `entity`; the SELECT maps missing-table → Ok(0).
    let db = open_memory().await.expect("open in-memory db");
    let n = delete_anti_patterns_by_gate(&db, "capture_finding_unpersisted")
        .await
        .expect("missing entity table is the empty case, not an error");
    assert_eq!(n, 0);
}

#[tokio::test]
async fn purge_by_gate_removes_the_cluster_and_leaves_others() {
    let db = open_memory().await.expect("open in-memory db");
    // Two distinct anti_pattern clusters by gate.
    seed(&db, "capture_finding_unpersisted", "old fix")
        .await
        .expect("seed target");
    seed(&db, "shallow_verdict", "cite file:line")
        .await
        .expect("seed bystander");
    assert_eq!(top_anti_patterns(&db, 10).await.expect("read").len(), 2);

    // Purge only the target gate.
    let removed = delete_anti_patterns_by_gate(&db, "capture_finding_unpersisted")
        .await
        .expect("purge target gate");
    assert_eq!(removed, 1, "exactly one anti_pattern cluster removed");

    // The bystander survives; the target is gone.
    let after = top_anti_patterns(&db, 10).await.expect("read after purge");
    assert_eq!(after.len(), 1, "only the bystander remains: {after:?}");
    assert_eq!(after[0].gate, "shallow_verdict");
}
