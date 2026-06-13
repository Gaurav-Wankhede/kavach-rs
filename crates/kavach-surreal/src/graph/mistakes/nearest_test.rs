// Proves cosine relevance-gating for the [MISTAKE_GUARD] pre-write frame:
// nearest_anti_patterns ranks the anti_pattern whose centroid is closest to the
// query embedding first, and the floor drops sub-threshold (irrelevant) hits.
use super::nearest_anti_patterns;
use crate::graph::mistakes::upsert_anti_pattern;
use crate::open_memory;

// 384-dim BGE-shaped unit vector with a single hot dimension: same index ⇒
// cosine 1.0, different index ⇒ cosine 0.0.
fn unit_vec(hot: usize) -> Vec<f32> {
    let mut v = vec![0.0_f32; 384];
    v[hot] = 1.0;
    v
}

#[tokio::test]
async fn cosine_nearest_ranks_the_relevant_anti_pattern_first() {
    let db = open_memory().await.expect("open in-memory db");
    upsert_anti_pattern(&db, "anti.a", "gate_a", "do A instead", &unit_vec(0))
        .await
        .expect("seed A");
    upsert_anti_pattern(&db, "anti.b", "gate_b", "do B instead", &unit_vec(1))
        .await
        .expect("seed B");

    // Query identical to A's centroid ⇒ A ranks first with cosine ~1.0.
    let hits = nearest_anti_patterns(&db, &unit_vec(0), 5, 0.5)
        .await
        .expect("nearest");
    assert_eq!(
        hits.first().map(|h| h.gate.as_str()),
        Some("gate_a"),
        "most-relevant anti-pattern must rank first, got {hits:?}"
    );
    assert!(
        hits[0].score > 0.99,
        "identical embedding ⇒ cosine ~1.0, got {}",
        hits[0].score
    );
}

#[tokio::test]
async fn floor_filters_out_irrelevant_hits() {
    let db = open_memory().await.expect("open in-memory db");
    upsert_anti_pattern(&db, "anti.a", "gate_a", "do A instead", &unit_vec(0))
        .await
        .expect("seed");
    // Query orthogonal to the only pattern ⇒ cosine 0.0 < floor 0.5 ⇒ filtered.
    let hits = nearest_anti_patterns(&db, &unit_vec(7), 5, 0.5)
        .await
        .expect("nearest");
    assert!(
        hits.is_empty(),
        "orthogonal query below floor ⇒ no hits, got {hits:?}"
    );
}

#[tokio::test]
async fn empty_graph_returns_no_hits() {
    let db = open_memory().await.expect("open in-memory db");
    let hits = nearest_anti_patterns(&db, &unit_vec(0), 5, 0.5)
        .await
        .expect("nearest");
    assert!(hits.is_empty(), "no anti_patterns ⇒ empty, got {hits:?}");
}

// Unnormalised multi-hot vector; SurrealDB's cosine normalises, so overlap with
// the query degrades gracefully (1 shared dim of N ⇒ cosine 1/sqrt(N)).
fn multi_hot(hots: &[usize]) -> Vec<f32> {
    let mut v = vec![0.0_f32; 384];
    for &h in hots {
        v[h] = 1.0;
    }
    v
}

#[tokio::test]
async fn k_caps_count_and_orders_by_descending_score() {
    let db = open_memory().await.expect("open in-memory db");
    upsert_anti_pattern(&db, "anti.a", "gate_a", "a", &multi_hot(&[0]))
        .await
        .expect("a"); // cosine ~1.0
    upsert_anti_pattern(&db, "anti.b", "gate_b", "b", &multi_hot(&[0, 1]))
        .await
        .expect("b"); // cosine ~0.707
    upsert_anti_pattern(&db, "anti.c", "gate_c", "c", &multi_hot(&[0, 1, 2]))
        .await
        .expect("c"); // cosine ~0.577
    upsert_anti_pattern(&db, "anti.d", "gate_d", "d", &multi_hot(&[3]))
        .await
        .expect("d"); // cosine 0.0

    // k=2 over 4 patterns: exactly the two highest, most-relevant first.
    let hits = nearest_anti_patterns(&db, &multi_hot(&[0]), 2, 0.0)
        .await
        .expect("nearest");
    assert_eq!(hits.len(), 2, "k must cap the count, got {hits:?}");
    assert_eq!(hits[0].gate, "gate_a", "highest score first, got {hits:?}");
    assert_eq!(hits[1].gate, "gate_b", "second-highest next, got {hits:?}");
    assert!(
        hits[0].score > hits[1].score,
        "descending order, got {hits:?}"
    );
}

#[tokio::test]
async fn floor_keeps_above_and_drops_below_in_one_call() {
    let db = open_memory().await.expect("open in-memory db");
    upsert_anti_pattern(&db, "anti.hi", "gate_hi", "hi", &multi_hot(&[0]))
        .await
        .expect("hi"); // ~1.0
    upsert_anti_pattern(&db, "anti.lo", "gate_lo", "lo", &multi_hot(&[0, 1, 2]))
        .await
        .expect("lo"); // ~0.577

    // floor 0.7 keeps the ~1.0 hit and drops the ~0.577 hit in the same call.
    let hits = nearest_anti_patterns(&db, &multi_hot(&[0]), 5, 0.7)
        .await
        .expect("nearest");
    assert_eq!(
        hits.len(),
        1,
        "only the above-floor hit survives, got {hits:?}"
    );
    assert_eq!(hits[0].gate, "gate_hi", "got {hits:?}");
}
