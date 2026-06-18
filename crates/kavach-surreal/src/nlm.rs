//! `NanoLM` live-doc corpus: idempotent chunk upsert + vectorless BM25 retrieval.
//!
//! The `NanoLM` never trusts frozen weights — fetched official-docs chunks land in
//! `nlm_doc` and are ranked live by full-text `search::score`, no embeddings.
use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::SurrealValue;

use crate::error::Result;

/// One BM25-ranked chunk returned from the `nlm_doc` corpus.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SurrealValue)]
#[non_exhaustive]
pub struct NlmHit {
    pub source_url: String,
    pub heading: String,
    pub body: String,
    pub score: f64,
}

/// Insert-or-update one doc chunk, keyed `(source_url, heading)` so re-fetching a
/// page never duplicates rows.
///
/// # Errors
/// Propagates `Error::Surreal` when the upsert query fails.
pub async fn upsert_doc(
    db: &Surreal<Db>,
    source_url: &str,
    heading: &str,
    body: &str,
    captured_at: &str,
) -> Result<()> {
    let q = "UPSERT nlm_doc \
             SET source_url = $url, heading = $heading, body = $body, \
                 captured_at = $captured_at, updated_at = time::now() \
             WHERE source_url = $url AND heading = $heading";
    db.query(q)
        .bind(("url", source_url.to_owned()))
        .bind(("heading", heading.to_owned()))
        .bind(("body", body.to_owned()))
        .bind(("captured_at", captured_at.to_owned()))
        .await?;
    Ok(())
}

/// BM25 retrieval over `nlm_doc.body`. Terms bind to `$terms` (never
/// interpolated — injection-safe); ranked by `search::score(0)` descending.
///
/// # Errors
/// Propagates `Error::Surreal` when the query fails.
pub async fn query_docs(db: &Surreal<Db>, terms: &str, limit: usize) -> Result<Vec<NlmHit>> {
    if terms.trim().is_empty() {
        return Ok(Vec::new());
    }
    let lim = i64::try_from(limit.clamp(1, 100)).unwrap_or(25);
    let q = "SELECT source_url, heading, body, search::score(0) AS score \
             FROM nlm_doc WHERE body @0@ $terms \
             ORDER BY score DESC LIMIT $limit";
    let mut resp = db
        .query(q)
        .bind(("terms", terms.to_owned()))
        .bind(("limit", lim))
        .await?;
    Ok(resp.take(0)?)
}
