//! `NanoLM` live-doc corpus: idempotent chunk upsert + vectorless BM25 retrieval.
//!
//! The `NanoLM` never trusts frozen weights — fetched official-docs chunks land in
//! `nlm_doc` and are ranked live by FTS5 BM25, no embeddings.
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::turso::TursoStore;

/// One BM25-ranked chunk returned from the `nlm_doc` corpus.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NlmHit {
    pub source_url: String,
    pub heading: String,
    pub body: String,
    pub score: f64,
}

/// Insert-or-update one doc chunk, keyed `(source_url, heading)` so re-fetching a
/// page never duplicates rows. Keeps the FTS index in sync with the base row.
///
/// # Errors
/// Propagates `Error::Turso` when the upsert query fails.
pub async fn upsert_doc(
    db: &TursoStore,
    source_url: &str,
    heading: &str,
    body: &str,
    captured_at: &str,
) -> Result<()> {
    let now = crate::turso::now_rfc3339();
    db.exec_params(
        "INSERT INTO nlm_doc (source_url, heading, body, captured_at, updated_at) \
         VALUES ($url, $heading, $body, $captured_at, $now) \
         ON CONFLICT(source_url, heading) DO UPDATE SET \
             body = excluded.body, captured_at = excluded.captured_at, \
             updated_at = excluded.updated_at",
        libsql::params![
            source_url.to_owned(),
            heading.to_owned(),
            body.to_owned(),
            captured_at.to_owned(),
            now
        ],
    )
    .await?;
    // FTS5 has no UPSERT; delete-then-insert keeps the index row-faithful.
    db.exec_params(
        "DELETE FROM nlm_doc_fts WHERE source_url = $url AND heading = $heading",
        libsql::params![source_url.to_owned(), heading.to_owned()],
    )
    .await?;
    db.exec_params(
        "INSERT INTO nlm_doc_fts (source_url, heading, body) VALUES ($url, $heading, $body)",
        libsql::params![source_url.to_owned(), heading.to_owned(), body.to_owned()],
    )
    .await?;
    Ok(())
}

/// BM25 retrieval over `nlm_doc.body`. Terms bind to `$terms` (never
/// interpolated — injection-safe); ranked by FTS5 bm25() ascending (lower is
/// better), negated for a positive score.
///
/// # Errors
/// Propagates `Error::Turso` when the query fails.
pub async fn query_docs(db: &TursoStore, terms: &str, limit: usize) -> Result<Vec<NlmHit>> {
    if terms.trim().is_empty() {
        return Ok(Vec::new());
    }
    let lim = i64::try_from(limit.clamp(1, 100)).unwrap_or(25);
    let mut resp = db
        .exec_params(
            "SELECT d.source_url, d.heading, d.body, -bm25(nlm_doc_fts) AS score \
             FROM nlm_doc_fts \
             JOIN nlm_doc d ON d.source_url = nlm_doc_fts.source_url \
                            AND d.heading = nlm_doc_fts.heading \
             WHERE nlm_doc_fts MATCH $terms \
             ORDER BY score DESC LIMIT $limit",
            libsql::params![terms.to_owned(), lim],
        )
        .await?;
    let mut out = Vec::new();
    while let Some(r) = resp.next().await? {
        out.push(NlmHit {
            source_url: r.get::<String>(0)?,
            heading: r.get::<String>(1)?,
            body: r.get::<String>(2)?,
            score: r.get::<f64>(3)?,
        });
    }
    Ok(out)
}
