// Relevance-gated anti_pattern retrieval — the read side of the [MISTAKE_GUARD]
// pre-write frame (loop-eng F2). Where `top_anti_patterns` ranks by recurrence
// for SessionStart, this ranks by COSINE similarity to a query embedding (the
// file content about to be written), so the gate surfaces the mistakes most
// relevant to THIS edit at the point of action — Reflexion negatives, ERL
// selective retrieval. Reuses the same `vector::similarity::cosine` over the
// HNSW-indexed `entity.embedding` that `cluster::nearest_anti_pattern` uses.
//
// ALGO: cosine k-NN over the HNSW-indexed anti_pattern centroid set (one node
//   per behavioral cluster — dozens), top-k above a relevance floor. SurrealDB
//   evaluates the index-backed similarity + ORDER BY score DESC LIMIT k; Rust
//   only drops sub-floor hits. TIME: O(log N) index probe + O(k). SPACE: O(k).
//   YEAR: 2026.
use crate::error::Result;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::SurrealValue;

/// One cosine-relevant anti-pattern hit: the behavioral lesson plus its
/// similarity to the query embedding (1.0 = identical, 0.0 = orthogonal).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct AntiPatternHit {
    /// Gate that fired the originating mistakes.
    pub gate: String,
    /// The do-instead rule reinjected as a negative example (anti-parrot framing).
    pub correct_action: String,
    /// Cosine similarity to the query embedding, in `[0.0, 1.0]`.
    pub score: f32,
}

/// Top-`k` anti-patterns whose centroid is cosine-nearest the `query_embedding`,
/// keeping only hits at or above `floor` (sub-floor matches are noise at the
/// point of action). Ordered most-relevant first.
///
/// # Errors
/// Propagates `Error::Surreal` on a real query failure. A brand-new graph with
/// no `entity` table yet is the empty case (zero `anti_patterns`), not an error.
pub async fn nearest_anti_patterns(
    db: &Surreal<Db>,
    query_embedding: &[f32],
    k: usize,
    floor: f32,
) -> Result<Vec<AntiPatternHit>> {
    #[derive(SurrealValue)]
    struct Row {
        gate: String,
        correct_action: String,
        score: f32,
    }

    let q = "SELECT properties.gate AS gate, \
             properties.correct_action AS correct_action, \
             vector::similarity::cosine(embedding, $q) AS score \
             FROM entity \
             WHERE entity_type = 'anti_pattern' AND embedding IS NOT NONE \
             ORDER BY score DESC LIMIT $k";
    let k_i64 = i64::try_from(k).unwrap_or(i64::MAX);
    let mut resp = db
        .query(q)
        .bind(("q", query_embedding.to_vec()))
        .bind(("k", k_i64))
        .await?;
    // Missing `entity` table (graph never written) is the empty case, not a
    // failure — same contract as `top_anti_patterns`.
    let rows: Vec<Row> = match resp.take(0) {
        Ok(rows) => rows,
        Err(e) if crate::error::is_missing_table_error(&e) => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    Ok(rows
        .into_iter()
        .filter(|r| r.score >= floor)
        .map(|r| AntiPatternHit {
            gate: r.gate,
            correct_action: r.correct_action,
            score: r.score,
        })
        .collect())
}

#[cfg(test)]
#[path = "nearest_test.rs"]
mod nearest_test;
