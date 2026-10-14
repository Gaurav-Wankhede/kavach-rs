//! Integration proof for the Brain-OS Gap-1 `BM25`/FTS retrieval corpus
//! (roadmap.unit.harness.brain-os.g1a). Proves against a real embedded libSQL
//! store — with the PRODUCTION schema applied — that the FTS5 index on the
//! unified `memories_fts` table actually matches and ranks by BM25 relevance.
//! This is the no-vector retrieval side of hybrid search: the ONNX embedder was
//! removed (decision/onnx-removal-dag-rlaif-only), so keyword relevance is the
//! surviving lexical signal.
//!
//! If the FTS DDL syntax were wrong, `open_memory` here would error and every
//! test below would fail at setup — so setup itself is a witness that the schema
//! applies cleanly.
#![allow(
    clippy::expect_used,
    clippy::unwrap_in_result,
    reason = "test assertions: a panic on the Err/None path IS the failure signal"
)]

use kavach_db::db::DbStore;
use kavach_db::db::search::SearchStore;
use kavach_db::{open_memory, upsert_entry_full};

/// In-memory db with the production schema (incl. the FTS5 index) applied.
async fn db_with_schema() -> DbStore {
    open_memory().await.expect("memory db")
}

/// Seed a decision row through the PRODUCTION write path, so the FTS proof runs
/// over rows shaped exactly as `kavach db write` produces them.
async fn seed_decision(db: &DbStore, key: &str, title: &str, content: &str) {
    let proj = "fts-test".to_owned();
    upsert_entry_full()
        .db(db)
        .category("decision")
        .project_id(&proj)
        .entry_key(key)
        .title(title)
        .content(content)
        .event_source("test")
        .qualified_name("")
        .references(&[])
        .build_for_call()
        .await
        .expect("seed decision");
}

/// FTS-match proof: only the decision whose content carries the query terms
/// matches; the unrelated row stays out.
#[tokio::test]
async fn bm25_matches_content_on_the_decision_corpus() {
    let db = db_with_schema().await;
    seed_decision(
        &db,
        "d1",
        "hybrid retrieval design",
        "BM25 full-text fused with graph proximity, no vectors after the embedder removal",
    )
    .await;
    seed_decision(
        &db,
        "d2",
        "unrelated",
        "a note about lease heartbeats and fencing",
    )
    .await;

    let hits = db
        .fts_rank_category("fts-test", "decision", "embedder retrieval", 10)
        .await
        .expect("fts query");
    assert_eq!(
        hits,
        vec!["d1".to_owned()],
        "only the relevant decision matches, got {hits:?}"
    );
}

/// BM25 ranking proof: the denser doc ranks first, not insertion order.
#[tokio::test]
async fn bm25_score_ranks_more_relevant_first() {
    let db = db_with_schema().await;
    seed_decision(
        &db,
        "hot",
        "retrieval retrieval retrieval",
        "retrieval retrieval retrieval embedder retrieval",
    )
    .await;
    seed_decision(
        &db,
        "cold",
        "retrieval mention",
        "one passing retrieval reference here",
    )
    .await;

    let hits = db
        .fts_rank_category("fts-test", "decision", "retrieval", 10)
        .await
        .expect("scored fts query");
    assert_eq!(hits.len(), 2, "both decisions mention the term");
    assert_eq!(
        hits.first().map(String::as_str),
        Some("hot"),
        "denser doc ranks first, got {hits:?}"
    );
}

#[tokio::test]
async fn fresh_corpus_matches_nothing() {
    let db = db_with_schema().await;
    let hits = db
        .fts_rank_category("fts-test", "decision", "anything", 10)
        .await
        .expect("fts query on empty corpus");
    assert!(hits.is_empty(), "no rows => no hits, not an error");
}
