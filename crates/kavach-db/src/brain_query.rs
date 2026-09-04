//! Live retrieval backing `KavachBrain::search`/`gap` (Brain-OS G2, `kavach think`).
//!
//! Runs BM25/FULLTEXT (g1a) over each typed memory corpus, plus concept-graph
//! FTS, then fuses the per-source rank lists with RRF (g1b/`hybrid_search`). No
//! vectors — the ONNX embedder was removed (decision/onnx-removal-dag-rlaif-only),
//! so the lexical BM25 rank and the concept-graph rank ARE the two signals.
//!
//! FULLTEXT only resolves on the kv-rocksdb server (kv-mem does not serve it),
//! so the live witness is the running daemon, not an in-memory unit test
//! (research.surrealdb-3.1-fulltext-bm25-syntax).
use crate::brain::{BrainHit, GapReport, hybrid_search};
use crate::error::Result;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;

/// Typed memory tables carrying the g1a `title`+`content` FULLTEXT indexes.
const CORPUS_TABLES: [&str; 5] = ["decision", "roadmap", "research", "pattern", "app_spec"];

/// One BM25-ranked id list per corpus table, most-relevant first.
async fn fts_rank_one(db: &Surreal<Db>, table: &str, q: &str) -> Result<Vec<String>> {
    // `@1@` binds the content match to index ref 1 so search::score(1) reads its
    // BM25; title carries ref 0. A title-only or content-only match leaves the
    // other score NONE, so `?? 0` coalesces it — without that the arithmetic on
    // NONE fails. The combined score is aliased so ORDER BY can name it (ORDER BY
    // rejects a bare parenthesized expression).
    //
    // rid is the bare `entry_key` string field — NOT `record::id`/`<string>id`,
    // because these rows carry a composite record id (project + entry_key) whose
    // key part stringifies with a `String("…")` debug wrapper. `entry_key` is
    // already category-qualified (e.g. `decision.harness.kavach-brain-os`), so it
    // is the clean, citable key on its own — no table prefix (that double-prefixes
    // the category the key already carries).
    let sql = format!(
        "SELECT entry_key AS rid, \
         (search::score(0) ?? 0) + (search::score(1) ?? 0) AS sc \
         FROM {table} WHERE title @0@ $q OR content @1@ $q \
         ORDER BY sc DESC LIMIT $limit"
    );
    let mut resp = db
        .query(sql)
        .bind(("q", q.to_owned()))
        .bind(("limit", 25_i64))
        .await?;
    Ok(resp.take("rid")?)
}

/// Concept-graph FTS rank source: BM25 over the concept description corpus,
/// returning keyed `entity:…` ids most-relevant first.
async fn concept_rank_one(db: &Surreal<Db>, q: &str) -> Result<Vec<String>> {
    // Score aliased so ORDER BY can name it (a bare `search::score(0)` in ORDER
    // BY is rejected as an unexpected token). `<string>id` renders the canonical
    // `entity:…` form for citation parity with the corpus tables.
    let sql = "SELECT <string>id AS rid, search::score(0) AS sc FROM entity \
               WHERE entity_type = 'concept' AND properties.description @0@ $q \
               ORDER BY sc DESC LIMIT $limit";
    let mut resp = db
        .query(sql)
        .bind(("q", q.to_owned()))
        .bind(("limit", 25_i64))
        .await?;
    Ok(resp.take("rid")?)
}

/// Hybrid keyword+graph retrieval: BM25 over every corpus table + concept FTS,
/// fused by RRF and truncated to `limit`. Empty query yields no hits.
///
/// # Errors
/// Propagates `Error::Surreal` when any corpus query fails.
pub async fn search_corpus(db: &Surreal<Db>, query: &str, limit: usize) -> Result<Vec<BrainHit>> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut ranked: Vec<Vec<String>> = Vec::with_capacity(CORPUS_TABLES.len().saturating_add(1));
    for table in CORPUS_TABLES {
        ranked.push(fts_rank_one(db, table, query).await?);
    }
    // Concept-graph FTS as one more rank source. Returns keyed ids (`entity:…`)
    // for citation parity with the corpus tables.
    ranked.push(concept_rank_one(db, query).await?);

    let refs: Vec<&[String]> = ranked.iter().map(Vec::as_slice).collect();
    Ok(hybrid_search(&refs, limit))
}

/// Deterministic gap signal: a thin/empty corpus for the query IS the gap.
///
/// When fewer than `floor` hits surface, the query itself is the missing topic
/// — `think` auto-files it as a research card. No LLM judgment in this slice.
///
/// # Errors
/// Propagates `Error::Surreal` from the underlying retrieval.
pub async fn gap_for(db: &Surreal<Db>, query: &str, floor: usize) -> Result<GapReport> {
    let hits = search_corpus(db, query, floor.max(1)).await?;
    let missing = if hits.len() < floor {
        vec![query.trim().to_owned()]
    } else {
        Vec::new()
    };
    Ok(GapReport { missing })
}
