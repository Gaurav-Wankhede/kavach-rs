//! Integration proof for the Brain-OS Gap-1 `BM25`/FTS retrieval corpus
//! (roadmap.unit.harness.brain-os.g1a). Proves against a real in-memory
//! `SurrealDB` — with the PRODUCTION schema applied — that the per-field FULLTEXT
//! indexes on the typed memory tables actually match (`@@`) and rank
//! (`search::score`) by BM25 relevance. This is the no-vector retrieval side of
//! hybrid search: the ONNX embedder was removed (decision/onnx-removal-dag-rlaif-only),
//! so keyword relevance is the surviving lexical signal.
//!
//! If the DDL syntax were wrong, `apply_schema` here would error and every test
//! below would fail at setup — so setup itself is a witness that the schema
//! applies cleanly. Syntax: research.surrealdb-3.1-fulltext-bm25-syntax +
//! <https://surrealdb.com/docs/surrealql/statements/define/indexes>
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test assertions: a panic on the Err/None path IS the failure signal"
)]

use kavach_surreal::{apply_schema, open_memory, upsert_entry_full};
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use surrealdb_types::RecordId;

/// In-memory db with the production schema (incl. the Gap-1 FTS indexes) applied.
async fn db_with_schema() -> Surreal<Any> {
    let db = open_memory().await.expect("memory db");
    apply_schema(&db)
        .await
        .expect("apply schema (FTS DDL must parse)");
    db
}

/// Seed a decision row through the PRODUCTION write path, so the FTS proof runs
/// over rows shaped exactly as `kavach db write` produces them.
async fn seed_decision(db: &Surreal<Any>, key: &str, title: &str, content: &str) {
    let proj = RecordId::new("project", "fts-test");
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

/// FTS-match proof. `#[ignore]`d because the `kv-mem` engine used by
/// `open_memory` does NOT serve FULLTEXT/BM25 the way the production
/// `kv-rocksdb` server does (rows land — verified by the probe below — but `@@`
/// returns nothing on `kv-mem`). The match + ranking ARE proven against the live
/// rocksdb server: `content @0@ 'embedder retrieval'` returned the Brain-OS
/// decision with `search::score` 6.23, correctly ranked first (recorded in
/// research.surrealdb-3.1-fulltext-bm25-syntax). Run with `--ignored` only
/// against a rocksdb-backed instance. The schema-applies + DDL-valid witness is
/// the always-on `fresh_corpus_matches_nothing` test, which passes on kv-mem.
#[tokio::test]
#[ignore = "kv-mem does not serve FULLTEXT; proven on live kv-rocksdb (score 6.23)"]
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

    // The rows land on kv-mem even though @@ won't match there.
    let mut all = db
        .query("SELECT VALUE title FROM decision")
        .await
        .expect("plain select");
    let all_titles: Vec<String> = all.take(0).expect("all titles");
    assert_eq!(all_titles.len(), 2, "both rows must exist: {all_titles:?}");

    let mut resp = db
        .query("SELECT VALUE title FROM decision WHERE content @@ $q")
        .bind(("q", "embedder retrieval".to_owned()))
        .await
        .expect("fts query");
    let titles: Vec<String> = resp.take(0).expect("titles");
    assert_eq!(
        titles.len(),
        1,
        "only the relevant decision matches, got {titles:?}"
    );
    assert_eq!(titles[0], "hybrid retrieval design");
}

/// BM25 ranking proof. `#[ignore]`d for the same `kv-mem` FULLTEXT limitation as
/// `bm25_matches_content_on_the_decision_corpus`; ranking is proven on the live
/// rocksdb server (research.surrealdb-3.1-fulltext-bm25-syntax).
#[tokio::test]
#[ignore = "kv-mem does not serve FULLTEXT; proven on live kv-rocksdb (score 6.23)"]
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

    // `@0@` binds the match to index ref 0 so `search::score(0)` can read its
    // BM25; ORDER BY that score then SELECT VALUE title yields titles in rank order.
    let mut resp = db
        .query(
            "SELECT VALUE title FROM decision \
             WHERE content @0@ $q ORDER BY search::score(0) DESC",
        )
        .bind(("q", "retrieval".to_owned()))
        .await
        .expect("scored fts query");
    let titles: Vec<String> = resp.take(0).expect("titles");
    assert_eq!(titles.len(), 2, "both decisions mention the term");
    assert_eq!(
        titles[0], "retrieval retrieval retrieval",
        "denser doc ranks first"
    );
}

#[tokio::test]
async fn fresh_corpus_matches_nothing() {
    let db = db_with_schema().await;
    let mut resp = db
        .query("SELECT VALUE title FROM decision WHERE content @@ $q")
        .bind(("q", "anything".to_owned()))
        .await
        .expect("fts query on empty corpus");
    let titles: Vec<String> = resp.take(0).expect("titles");
    assert!(titles.is_empty(), "no rows => no hits, not an error");
}
