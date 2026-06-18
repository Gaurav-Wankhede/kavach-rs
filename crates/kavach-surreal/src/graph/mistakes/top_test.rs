// Regression test for top_anti_patterns: seeds two anti_pattern clusters via the
// REAL capture path (append_mistake_event + cluster_event_to_pattern) on an
// in-memory SurrealDB, then asserts the read query ranks them by recurrence.
// Proves the SurrealQL (`properties.gate AS gate`, multi-row
// `count(<-instance_of<-entity)`, Rust-side rank) actually works end to end —
// the read side of the loop the daemon writes.
use super::top_anti_patterns;
use crate::error::Result;
use crate::graph::mistakes::{append_mistake_event, cluster_event_to_pattern};
use crate::open_memory;

/// Route one mistake observation through the capture path under `gate`/`fix`.
/// Distinct (gate, fix) pairs cluster to distinct anti_patterns by content key
/// (`anti.<gate>.<blake3(fix)[..8]>`); the same pair re-clusters to the same node.
/// Helper returns `Result` and propagates with `?` — it asserts nothing, so it
/// does not trip `panic_in_result_fn` (which fires only on asserting bodies).
async fn seed(
    db: &surrealdb::Surreal<surrealdb::engine::any::Any>,
    gate: &str,
    fix: &str,
) -> Result<()> {
    let ev = append_mistake_event(db, gate, fix, "banned phrase", "sess", Some("proj")).await?;
    cluster_event_to_pattern(db, &ev, gate, fix).await?;
    Ok(())
}

#[tokio::test]
async fn ranks_anti_patterns_by_recurrence() {
    let db = open_memory().await.expect("open in-memory db");
    // Cluster A: 3 recurrences of the same behavioral mistake → hit_count 3.
    for _ in 0..3 {
        seed(&db, "gate_a", "do A instead")
            .await
            .expect("seed cluster A");
    }
    // Cluster B: 1 recurrence → hit_count 1.
    seed(&db, "gate_b", "do B instead")
        .await
        .expect("seed cluster B");

    let top = top_anti_patterns(&db, 10)
        .await
        .expect("read anti-patterns");
    assert_eq!(top.len(), 2, "two distinct clusters expected, got {top:?}");
    // Most-recurrent first.
    assert_eq!(top[0].gate, "gate_a", "A (3 hits) must rank first");
    assert_eq!(top[0].hit_count, 3);
    assert_eq!(top[0].correct_action, "do A instead");
    assert_eq!(top[1].gate, "gate_b");
    assert_eq!(top[1].hit_count, 1);
}

#[tokio::test]
async fn limit_truncates_to_top_n() {
    let db = open_memory().await.expect("open in-memory db");
    seed(&db, "gate_a", "fix a")
        .await
        .expect("seed a");
    seed(&db, "gate_b", "fix b")
        .await
        .expect("seed b");
    seed(&db, "gate_c", "fix c")
        .await
        .expect("seed c");

    let top = top_anti_patterns(&db, 2).await.expect("read anti-patterns");
    assert_eq!(top.len(), 2, "limit=2 must cap the result");
}

#[tokio::test]
async fn empty_graph_returns_no_rows() {
    let db = open_memory().await.expect("open in-memory db");
    let top = top_anti_patterns(&db, 10)
        .await
        .expect("read anti-patterns");
    assert!(top.is_empty(), "no anti_patterns ⇒ empty result");
}
