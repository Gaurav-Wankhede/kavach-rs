//! Live retrieval backing `KavachBrain::search`/`gap` (Brain-OS G2, `kavach think`).
//!
//! Runs BM25 (g1a) over each typed memory corpus via the `memories_fts` FTS5
//! index, plus concept-graph FTS, then fuses the per-source rank lists with RRF
//! (g1b/`hybrid_search`). No vectors — the ONNX embedder was removed
//! (decision/onnx-removal-dag-rlaif-only), so the lexical BM25 rank and the
//! concept-graph rank ARE the two signals.
use crate::brain::{BrainHit, GapReport, hybrid_search};
use crate::error::Result;
use crate::turso::TursoStore;
use crate::turso::search::fts5_query;

/// Typed memory categories carrying the g1a `title`+`content` FTS index.
const CORPUS_TABLES: [&str; 5] = ["decision", "roadmap", "research", "pattern", "app_spec"];

/// One BM25-ranked id list per corpus category, most-relevant first.
///
/// When `project` is supplied, matches belonging to `project` are ranked first
/// (with BM25 ranking), followed by relevant matches across other projects
/// to preserve hybrid cross-project knowledge recall without drowning out local context.
async fn fts_rank_one(
    db: &TursoStore,
    table: &str,
    safe_q: &str,
    project: Option<&str>,
) -> Result<Vec<String>> {
    // FTS5 bm25() returns lower-is-better; negate for a positive score. The
    // `entry_key` is already category-qualified (e.g. `decision.harness.kavach-brain-os`),
    // so it is the clean, citable key on its own.
    let mut resp = if let Some(proj) = project {
        db.exec_params(
            "SELECT m.entry_key AS rid, -bm25(memories_fts) AS sc \
             FROM memories_fts \
             JOIN memories m ON m.project = memories_fts.project \
                            AND m.category = memories_fts.category \
                            AND m.entry_key = memories_fts.entry_key \
             WHERE memories_fts.category = $cat AND memories_fts MATCH $q \
             ORDER BY (m.project = $proj) DESC, sc DESC LIMIT $limit",
            libsql::params![table.to_owned(), safe_q.to_owned(), proj.to_owned(), 25_i64],
        )
        .await?
    } else {
        db.exec_params(
            "SELECT m.entry_key AS rid, -bm25(memories_fts) AS sc \
             FROM memories_fts \
             JOIN memories m ON m.project = memories_fts.project \
                            AND m.category = memories_fts.category \
                            AND m.entry_key = memories_fts.entry_key \
             WHERE memories_fts.category = $cat AND memories_fts MATCH $q \
             ORDER BY sc DESC LIMIT $limit",
            libsql::params![table.to_owned(), safe_q.to_owned(), 25_i64],
        )
        .await?
    };
    let mut out = Vec::new();
    while let Some(r) = resp.next().await? {
        out.push(r.get::<String>(0)?);
    }
    Ok(out)
}

/// Concept-graph FTS rank source: BM25 over the concept description corpus,
/// returning keyed `entity:…` ids most-relevant first.
async fn concept_rank_one(db: &TursoStore, q: &str) -> Result<Vec<String>> {
    // Concepts live in `entities` with `entity_type = 'concept'`; the description
    // is inside the JSON `properties` column. FTS5 cannot index JSON text, so
    // fall back to a LIKE scan over the serialized properties (bounded by the
    // concept population, which is small).
    let mut resp = db
        .exec_params(
            "SELECT id AS rid FROM entities \
             WHERE entity_type = 'concept' AND properties LIKE $q \
             LIMIT $limit",
            libsql::params![format!("%{q}%"), 25_i64],
        )
        .await?;
    let mut out = Vec::new();
    while let Some(r) = resp.next().await? {
        out.push(r.get::<String>(0)?);
    }
    Ok(out)
}

/// Hybrid keyword+graph retrieval: BM25 over every corpus table + concept FTS,
/// fused by RRF and truncated to `limit`. Empty query yields no hits.
///
/// # Errors
/// Propagates `Error::Turso` when any corpus query fails.
pub async fn search_corpus(db: &TursoStore, query: &str, limit: usize) -> Result<Vec<BrainHit>> {
    search_corpus_project(db, None, query, limit).await
}

/// Hybrid keyword+graph retrieval scoped with project-priority boosting.
///
/// Query terms are safely escaped for FTS5 syntax, and results from `project`
/// are prioritized at the top of category lists before cross-project matches.
///
/// # Errors
/// Propagates `Error::Turso` when any corpus query fails.
pub async fn search_corpus_project(
    db: &TursoStore,
    project: Option<&str>,
    query: &str,
    limit: usize,
) -> Result<Vec<BrainHit>> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let safe_q = fts5_query(trimmed);
    if safe_q.is_empty() {
        return Ok(Vec::new());
    }

    let mut ranked: Vec<Vec<String>> = Vec::with_capacity(CORPUS_TABLES.len().saturating_add(1));
    for table in CORPUS_TABLES {
        ranked.push(fts_rank_one(db, table, &safe_q, project).await?);
    }
    // Concept-graph FTS as one more rank source. Returns keyed ids (`entity:…`)
    // for citation parity with the corpus tables.
    ranked.push(concept_rank_one(db, trimmed).await?);

    let refs: Vec<&[String]> = ranked.iter().map(Vec::as_slice).collect();
    Ok(hybrid_search(&refs, limit))
}

/// Deterministic gap signal: a thin/empty corpus for the query IS the gap.
///
/// When fewer than `floor` hits surface, the query itself is the missing topic
/// — `think` auto-files it as a research card. No LLM judgment in this slice.
///
/// # Errors
/// Propagates `Error::Turso` from the underlying retrieval.
pub async fn gap_for(db: &TursoStore, query: &str, floor: usize) -> Result<GapReport> {
    gap_for_project(db, None, query, floor).await
}

/// Deterministic gap signal scoped to an optional project.
///
/// # Errors
/// Propagates `Error::Turso` from the underlying retrieval.
pub async fn gap_for_project(
    db: &TursoStore,
    project: Option<&str>,
    query: &str,
    floor: usize,
) -> Result<GapReport> {
    let hits = search_corpus_project(db, project, query, floor.max(1)).await?;
    let missing = if hits.len() < floor {
        vec![query.trim().to_owned()]
    } else {
        Vec::new()
    };
    Ok(GapReport { missing })
}
