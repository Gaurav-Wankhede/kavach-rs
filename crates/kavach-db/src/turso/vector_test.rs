//! VectorStore tests — Phase 5 acceptance (PRD §4 Phase 5).
//!
//! Tests are deterministic: `TextEmbedding` model download is skipped in CI
//! (offline) so `embed_texts` falls back to `fallback_embed` (blake3 hash),
//! ensuring cosine scores are reproducible without a 120 MB model cache.

use super::TursoStore;
use super::entries::EntryStore;
use super::vector::{
    EMBEDDING_DIMS, VectorStore, cosine_similarity, deserialize_embedding, embed_texts,
    fallback_embed, serialize_embedding,
};
use crate::dual_write::MemoryEntry;

fn entry(project: &str, category: &str, key: &str, title: &str, content: &str) -> MemoryEntry {
    MemoryEntry {
        id: None,
        project: project.to_owned(),
        category: Some(category.to_owned()),
        entry_key: key.to_owned(),
        title: title.to_owned(),
        content: content.to_owned(),
        status: Some("todo".into()),
        entry_status: Some("todo".into()),
        tags: None,
        decay_score: None,
        access_count: Some(0),
        priority: None,
        lane: None,
        exec_prompt: None,
        occupied_by: None,
        occupied_until: None,
        created_at: None,
        updated_at: None,
    }
}

// ── Serialization round-trip ──────────────────────────────────────────────────

#[test]
fn serialize_deserialize_roundtrip() {
    let original: Vec<f32> = (0..EMBEDDING_DIMS).map(|i| i as f32 / 100.0).collect();
    let blob = serialize_embedding(&original);
    assert_eq!(
        blob.len(),
        EMBEDDING_DIMS * 4,
        "blob must be 4 bytes per f32"
    );
    let recovered = deserialize_embedding(&blob);
    assert_eq!(recovered.len(), EMBEDDING_DIMS);
    for (a, b) in original.iter().zip(recovered.iter()) {
        let diff = (a - b).abs();
        assert!(diff < f32::EPSILON, "round-trip mismatch: {a} vs {b}");
    }
}

#[test]
fn deserialize_malformed_length_returns_empty() {
    let malformed = vec![0u8; 5]; // 5 bytes — not a multiple of 4
    assert!(deserialize_embedding(&malformed).is_empty());
}

// ── Cosine similarity ─────────────────────────────────────────────────────────

#[test]
fn cosine_identical_vectors_is_one() {
    let v: Vec<f32> = fallback_embed("identical");
    let sim = cosine_similarity(&v, &v);
    let diff = (sim - 1.0_f32).abs();
    assert!(diff < 1e-4_f32, "cosine(v, v) should be ~1.0, got {sim}");
}

#[test]
fn cosine_different_texts_below_one() {
    let a = fallback_embed("rust ownership");
    let b = fallback_embed("potato salad recipe");
    let sim = cosine_similarity(&a, &b);
    assert!(sim <= 1.0 + 1e-6_f32, "cosine must be ≤ 1.0, got {sim}");
}

// ── Fallback embedding ────────────────────────────────────────────────────────

#[test]
fn fallback_embed_produces_correct_dimension() {
    let v = fallback_embed("hello world");
    assert_eq!(v.len(), EMBEDDING_DIMS);
}

#[test]
fn fallback_embed_is_deterministic() {
    let a = fallback_embed("determinism test");
    let b = fallback_embed("determinism test");
    assert_eq!(a, b, "fallback_embed must be deterministic");
}

#[test]
fn fallback_embed_is_unit_length() {
    let v = fallback_embed("unit sphere");
    let norm_sq: f32 = v.iter().map(|x| x * x).sum();
    let diff = (norm_sq - 1.0_f32).abs();
    assert!(
        diff < 1e-3_f32,
        "fallback vector must be unit length, norm²={norm_sq}"
    );
}

// ── embed_texts ───────────────────────────────────────────────────────────────

#[test]
fn embed_texts_empty_returns_empty() {
    let out = embed_texts(&[]).unwrap();
    assert!(out.is_empty());
}

#[test]
fn embed_texts_produces_correct_dimension() {
    let texts = vec!["hello".to_owned(), "world".to_owned()];
    let out = embed_texts(&texts).unwrap();
    assert_eq!(out.len(), 2);
    for v in &out {
        assert_eq!(
            v.len(),
            EMBEDDING_DIMS,
            "all vectors must be {EMBEDDING_DIMS}-D"
        );
    }
}

// ── Write-time embedding integration ─────────────────────────────────────────

#[tokio::test]
async fn upsert_entry_writes_embedding() {
    let dir = tempfile::TempDir::new().unwrap();
    let s = TursoStore::open(dir.path().join("vec_embed.db"))
        .await
        .unwrap();
    s.upsert_entry(&entry(
        "p",
        "decision",
        "d.embed",
        "Rust async",
        "tokio runtime",
    ))
    .await
    .unwrap();

    // Give spawn_blocking a moment then check embedding was written.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let mut rows = s
        .query_with_params(
            "SELECT embedding FROM memories WHERE entry_key = ?1",
            libsql::params!["d.embed"],
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().expect("row must exist");
    let blob: Option<Vec<u8>> = row.get(0).ok();
    assert!(blob.is_some(), "embedding must be non-NULL after upsert");
    let vec = deserialize_embedding(&blob.unwrap());
    assert_eq!(
        vec.len(),
        EMBEDDING_DIMS,
        "embedded vector must be {EMBEDDING_DIMS}-D"
    );
}

// ── VectorStore::vector_rank ──────────────────────────────────────────────────

#[tokio::test]
async fn vector_rank_returns_hits() {
    let dir = tempfile::TempDir::new().unwrap();
    let s = TursoStore::open(dir.path().join("vec_rank.db"))
        .await
        .unwrap();

    for (key, title, content) in [
        ("k.rust", "Rust ownership", "borrow checker and lifetimes"),
        ("k.tokio", "Async runtime", "tokio executor threads"),
        ("k.sql", "SQL indexing", "btree index scan"),
    ] {
        s.upsert_entry(&entry("proj", "decision", key, title, content))
            .await
            .unwrap();
    }
    // Allow spawn_blocking to complete.
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;

    let hits = s
        .vector_rank("proj", "Rust borrow checker", 5)
        .await
        .unwrap();
    assert!(!hits.is_empty(), "vector_rank must return at least one hit");
    // The top hit must be the Rust entry.
    assert_eq!(hits.first().map(String::as_str), Some("k.rust"));
}

#[tokio::test]
async fn vector_rank_empty_query_returns_empty() {
    let dir = tempfile::TempDir::new().unwrap();
    let s = TursoStore::open(dir.path().join("vec_empty.db"))
        .await
        .unwrap();
    let hits = s.vector_rank("proj", "  ", 5).await.unwrap();
    assert!(hits.is_empty());
}

// ── VectorStore::search_hybrid ────────────────────────────────────────────────

#[tokio::test]
async fn search_hybrid_returns_fused_results() {
    let dir = tempfile::TempDir::new().unwrap();
    let s = TursoStore::open(dir.path().join("vec_hybrid.db"))
        .await
        .unwrap();

    for (key, cat, title, content) in [
        (
            "decision.embed",
            "decision",
            "Embed libSQL",
            "replace surreal with libsql",
        ),
        (
            "roadmap.embed-task",
            "roadmap",
            "Migrate storage",
            "swap backend to turso",
        ),
        (
            "pattern.unrelated",
            "pattern",
            "Kanban lanes",
            "dispatch order priority",
        ),
    ] {
        s.upsert_entry(&entry("proj", cat, key, title, content))
            .await
            .unwrap();
    }
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;

    let hits = s
        .search_hybrid("proj", "embed libsql storage", 10)
        .await
        .unwrap();
    assert!(!hits.is_empty(), "hybrid search must return results");

    let ids: Vec<&str> = hits.iter().map(|h| h.id.as_str()).collect();
    // Both matching entries must surface.
    assert!(
        ids.contains(&"decision.embed"),
        "decision.embed must surface"
    );
    assert!(
        ids.contains(&"roadmap.embed-task"),
        "roadmap.embed-task must surface"
    );

    // Scores must be descending.
    let scores: Vec<f64> = hits.iter().map(|h| h.score).collect();
    let mut sorted = scores.clone();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());
    assert_eq!(scores, sorted, "hits must be in descending score order");
}

// ── VectorStore::backfill_embeddings ─────────────────────────────────────────

#[tokio::test]
async fn backfill_fills_null_embeddings() {
    let dir = tempfile::TempDir::new().unwrap();
    let s = TursoStore::open(dir.path().join("vec_backfill.db"))
        .await
        .unwrap();

    // Directly insert rows with NULL embedding to simulate pre-Phase5 rows.
    for key in ["bf.a", "bf.b", "bf.c"] {
        s.conn
            .execute(
                "INSERT INTO memories (project, category, entry_key, title, content, \
                 status, tags, access_count, created_at, updated_at) \
                 VALUES ('bp','decision',?1,'Title','Content','todo','[]',0,\
                 datetime('now'),datetime('now'))",
                libsql::params![key],
            )
            .await
            .unwrap();
    }

    // Verify NULL before backfill.
    let mut rows = s
        .query_with_params(
            "SELECT count(*) FROM memories WHERE embedding IS NULL AND project='bp'",
            (),
        )
        .await
        .unwrap();
    let null_count: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
    assert_eq!(
        null_count, 3,
        "should have 3 NULL embeddings before backfill"
    );

    let filled = s.backfill_embeddings(50).await.unwrap();
    assert_eq!(filled, 3, "backfill must fill all 3 rows");

    // Verify non-NULL after backfill.
    let mut rows = s
        .query_with_params(
            "SELECT count(*) FROM memories WHERE embedding IS NULL AND project='bp'",
            (),
        )
        .await
        .unwrap();
    let still_null: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
    assert_eq!(
        still_null, 0,
        "no NULL embeddings must remain after backfill"
    );
}
