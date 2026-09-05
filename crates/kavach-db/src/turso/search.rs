//! FTS5 search — `SearchStore` extension trait over `TursoStore`.
//!
//! Phase 2 parity with brain_query.rs: TursoStoreDB ran one BM25 query per
//! category table and produced one rank list per table. libSQL keeps all
//! categories in ONE fts5 table with a `category` column, so the equivalent is
//! one filtered query per category: bm25() ranks rows, RRF consumes order.

use std::future::Future;

use super::{TursoError, TursoResult, TursoStore};
use crate::brain::BrainHit;
use crate::rrf::{RRF_K, rrf_fuse};

/// Categories that carry searchable title/content corpora (mirrors
/// `CORPUS_TABLES` in brain_query.rs).
pub const CORPUS_CATEGORIES: [&str; 5] = ["decision", "roadmap", "research", "pattern", "app_spec"];

/// FTS-owned DDL, executed by `connection::init_schema`.
pub const DDL: &[&str] = &[
    "CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(entry_key UNINDEXED, project UNINDEXED, category UNINDEXED, title, content, tokenize='porter unicode61')",
];

/// Convert a user query string to FTS5 MATCH syntax, injection-safe.
///
/// TursoStoreDB `@0@` treats terms as an implicit AND of prefixes; FTS5 needs
/// explicit quoting so user punctuation can't break the expression. Each
/// whitespace-separated token is quoted (with embedded quotes doubled).
#[must_use]
pub fn fts5_query(query: &str) -> String {
    query
        .split_whitespace()
        .map(fts_escape)
        .collect::<Vec<_>>()
        .join(" ")
}

fn fts_escape(term: &str) -> String {
    format!("\"{}\"", term.replace('"', "\"\""))
}

/// Positional params for [`SearchStore::fts_rank_category`].
fn rank_params(project: &str, category: &str, query: &str, limit: usize) -> Vec<libsql::Value> {
    #[expect(
        clippy::cast_possible_wrap,
        reason = "limit is a CLI-bounded search cap (SEARCH_LIMIT_MAX), far below i64::MAX"
    )]
    let limit64 = limit as i64;
    vec![
        libsql::Value::Text(fts5_query(query)),
        libsql::Value::Text(project.to_owned()),
        libsql::Value::Text(category.to_owned()),
        libsql::Value::Integer(limit64),
    ]
}

/// BM25 + fusion retrieval on the embedded store.
pub trait SearchStore {
    /// One BM25 rank list: hit ids for `query` within `category`,
    /// most-relevant first. Parity of `brain_query::fts_rank_one`.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn fts_rank_category(
        &self,
        project: &str,
        category: &str,
        query: &str,
        limit: usize,
    ) -> impl Future<Output = TursoResult<Vec<String>>> + Send;

    /// Hybrid retrieval parity of `brain_query::search_corpus`: one rank list
    /// per corpus category, fused by RRF into [`BrainHit`]s.
    ///
    /// # Errors
    /// Returns [`TursoError`] when any category query fails.
    fn search_corpus(
        &self,
        project: &str,
        query: &str,
        limit: usize,
    ) -> impl Future<Output = TursoResult<Vec<BrainHit>>> + Send;
}

impl SearchStore for TursoStore {
    async fn fts_rank_category(
        &self,
        project: &str,
        category: &str,
        query: &str,
        limit: usize,
    ) -> TursoResult<Vec<String>> {
        // bm25() lower = better; ORDER BY bm25(memories_fts) ascending.
        let mut rows = self
            .exec_params(
                "SELECT entry_key FROM memories_fts \
                 WHERE memories_fts MATCH ?1 AND project = ?2 AND category = ?3 \
                 ORDER BY bm25(memories_fts) LIMIT ?4",
                rank_params(project, category, query, limit),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(TursoError::from)? {
            out.push(row.get::<String>(0).map_err(TursoError::from)?);
        }
        Ok(out)
    }

    async fn search_corpus(
        &self,
        project: &str,
        query: &str,
        limit: usize,
    ) -> TursoResult<Vec<BrainHit>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        let mut ranked: Vec<Vec<String>> = Vec::with_capacity(CORPUS_CATEGORIES.len());
        for cat in CORPUS_CATEGORIES {
            ranked.push(self.fts_rank_category(project, cat, query, limit).await?);
        }
        let refs: Vec<&[String]> = ranked.iter().map(Vec::as_slice).collect();
        Ok(rrf_fuse(&refs, RRF_K)
            .into_iter()
            .take(limit)
            .map(|(id, score)| BrainHit { id, score })
            .collect())
    }
}

/// Test-only exposure of the escaper (property-tested for injection safety).
#[cfg(test)]
#[expect(
    dead_code,
    reason = "search_test drives fts5_query directly; kept for API symmetry"
)]
pub(crate) fn fts5_query_for_tests(q: &str) -> String {
    fts5_query(q)
}
