//! Turso-backed citation store: official-docs context rows keyed by
//! (project, entry_key) via the UNIQUE idx_citation_project_key. Mirrors the
//! gate_patterns CRUD idiom — explicit column list, bound params, row mapping
//! via `next()` + typed `get`. Datetimes project through RFC3339 TEXT columns.
//! sql-safe: static literals + bound params only; no string concat.
use crate::error::Result;
pub use crate::graph::{citations_cited_by, traverse_with_citations as traverse};
use crate::turso::TursoStore;
use serde::{Deserialize, Serialize};
/// Non-destructively merge an existing learning-tier node (mistake / anti-pattern
/// / decision / roadmap) into the citation DAG by relating it `->cite->` the
/// citation it draws on.
///
/// The node's own table is untouched — the merge is an EDGE, never a move. RLAIF
/// reward (C8) and recall (C7) then flow along this edge.
///
/// Idempotent: a node already merged into the citation is a no-op, so a replayed
/// merge never duplicates the `cite` edge (which would double-count C8 reward).
///
/// # Errors
/// Propagates the `relate_citation` error (rejects a non-`cite`/`parent`/
/// `depends_on` edge, or a DB failure).
pub async fn merge_node_into_citation(db: &TursoStore, node: &str, citation: &str) -> Result<()> {
    let mut response = db
        .exec_params(
            "SELECT from_id FROM entity_edges WHERE from_id = $node AND rel_type = 'cite' AND to_id = $cit",
            libsql::params![node.to_owned(), citation.to_owned()],
        )
        .await?;
    // The `cite` edge table is absent until the first RELATE creates it; a
    // missing table is the "no edge yet" case, not a failure.
    let mut existing = Vec::new();
    while let Some(row) = response.next().await? {
        existing.push(row.get::<String>(0)?);
    }
    if existing.is_empty() {
        crate::graph::relate_citation(db, node, citation, "cite", 1.0).await?;
    }
    Ok(())
}
/// Recall bridge: for a batch of recalled decision/roadmap nodes, gather every
/// citation they cite in ONE round-trip, deduped and order-stable.
///
/// This is how decision + roadmap recall is wired through the citation DAG — the
/// `on_prompt` gate injects these citations alongside the recalled rows so the
/// model sees the official-docs context the decision rested on.
///
/// # Errors
/// Propagates `Error::Turso` from the traversal.
pub async fn citations_for_nodes(db: &TursoStore, nodes: &[String]) -> Result<Vec<String>> {
    if nodes.is_empty() {
        return Ok(Vec::new());
    }
    // One round-trip per node: SQLite has no array bind, so expand to a
    // parameterized IN-list (bounded by caller batch size).
    let placeholders = nodes.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!(
        "SELECT to_id FROM entity_edges WHERE rel_type = 'cite' AND from_id IN ({placeholders})"
    );
    let mut response = db.exec_params(&sql, nodes.to_vec()).await?;
    let mut out: Vec<String> = Vec::new();
    while let Some(row) = response.next().await? {
        let cit = row.get::<String>(0)?;
        if !out.contains(&cit) {
            out.push(cit);
        }
    }
    Ok(out)
}
/// RLAIF reward: when a citation contributed to a successful turn, flow `delta`
/// reward along every `cite` edge pointing into it, bumping each edge `weight`.
///
/// Returns the number of edges rewarded. Reward accrues on the EDGE (the
/// node→citation link), so a node that repeatedly draws on a citation that keeps
/// helping climbs in recall ranking — the self-improving loop closes here.
///
/// # Errors
/// Propagates `Error::Turso` from the UPDATE.
pub async fn reward_citation_edges(db: &TursoStore, citation: &str, delta: f64) -> Result<usize> {
    let mut response = db
        .exec_params(
            "UPDATE entity_edges SET weight = weight + $delta WHERE rel_type = 'cite' AND to_id = $cit RETURNING from_id",
            libsql::params![delta, citation.to_owned()],
        )
        .await?;
    let mut updated = 0usize;
    while response.next().await?.is_some() {
        updated += 1;
    }
    Ok(updated)
}
/// Seconds in the freshness window.
///
/// A citation whose `updated_at` is older than this is STALE and must be
/// re-researched against the official docs (C5). 24h — versions/APIs move fast
/// and weights are never trusted, so re-confirm daily.
pub const FRESHNESS_WINDOW_SECS: i64 = 24 * 60 * 60;
/// The `[STALE]` marker prepended to a stale citation's injected text so the
/// model treats served-stale content with suspicion until C5 refreshes it.
pub const STALE_MARKER: &str = "[STALE]";
/// Freshness verdict for a citation row, decided purely from its `updated_unix`
/// and a caller-supplied `now` epoch — no clock read, no I/O, fully testable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Freshness {
    /// Updated within `FRESHNESS_WINDOW_SECS` of `now`.
    Fresh,
    /// Older than the window, or missing `updated_unix` (legacy/never-stamped).
    Stale,
}
/// Classify a citation by age. `None`/future `updated_unix` and any age beyond
/// the window are `Stale` (fail-suspicious: an unstamped row is not trusted).
#[must_use]
pub const fn freshness(updated_unix: Option<i64>, now: i64) -> Freshness {
    match updated_unix {
        Some(ts) if now.saturating_sub(ts) < FRESHNESS_WINDOW_SECS && ts <= now => Freshness::Fresh,
        _ => Freshness::Stale,
    }
}
/// Prefix `text` with `STALE_MARKER` when the verdict is `Stale`, else return it
/// unchanged — the one place injection decides whether to flag served content.
#[must_use]
pub fn mark_if_stale(verdict: Freshness, text: &str) -> String {
    match verdict {
        Freshness::Stale => format!("{STALE_MARKER} {text}"),
        Freshness::Fresh => text.to_owned(),
    }
}
/// One stale citation queued for re-research: its `entry_key` plus every
/// official-docs URL to re-fetch.
///
/// The harness/RPC (C9) drives the actual WebSearch/WebFetch; this struct is the
/// pure work-list it consumes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RefreshTarget {
    pub entry_key: String,
    pub urls: Vec<String>,
}
/// On-recall lazy refresh plan: which recalled citations are stale and must be
/// re-researched THIS turn, and the marker-decorated text to serve meanwhile.
///
/// Pure — the caller injects `now` and executes the fetches.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RefreshPlan {
    pub refresh: Vec<RefreshTarget>,
    pub served: Vec<String>,
}
/// Partition recalled citations by freshness.
///
/// Stale ones become `refresh` targets (queued for re-research) AND are still
/// served with the `[STALE]` marker so the turn never blocks on the network;
/// fresh ones serve clean.
#[must_use]
pub fn plan_refresh(citations: &[Citation], now: i64) -> RefreshPlan {
    let mut refresh = Vec::new();
    let mut served = Vec::with_capacity(citations.len());
    for c in citations {
        let verdict = freshness(c.updated_unix, now);
        served.push(mark_if_stale(verdict, &c.name));
        if verdict == Freshness::Stale {
            refresh.push(RefreshTarget {
                entry_key: c.entry_key.clone(),
                urls: c.metadata.iter().map(|m| m.url.clone()).collect(),
            });
        }
    }
    RefreshPlan { refresh, served }
}
const COLS: &str = "project, entry_key, name, metadata, access_count, \
                    created_at, updated_at";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CitationMeta {
    pub slug: String,
    #[serde(default)]
    pub desc: String,
    pub url: String,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub depends_on: Option<String>,
    #[serde(default)]
    pub best_practice: String,
    #[serde(default)]
    pub worst_practice: String,
    #[serde(default)]
    pub tradeoff: String,
}
impl CitationMeta {
    #[must_use]
    pub const fn new(slug: String, url: String) -> Self {
        Self {
            slug,
            desc: String::new(),
            url,
            parent: None,
            depends_on: None,
            best_practice: String::new(),
            worst_practice: String::new(),
            tradeoff: String::new(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Citation {
    pub id: Option<String>,
    pub project: String,
    pub entry_key: String,
    pub name: String,
    #[serde(default)]
    pub metadata: Vec<CitationMeta>,
    #[serde(default)]
    pub access_count: i64,
    #[serde(default)]
    pub created_unix: Option<i64>,
    #[serde(default)]
    pub updated_unix: Option<i64>,
}
#[derive(Debug)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate struct-literal DTO (kavach-rpc); non_exhaustive => E0639"
)]
pub struct UpsertCitation<'a> {
    pub project: String,
    pub entry_key: &'a str,
    pub name: &'a str,
    pub metadata: Vec<CitationMeta>,
}
/// Map a citation row (column order = [`COLS`]) into a [`Citation`].
fn citation_from_row(row: &libsql::Row) -> Result<Citation> {
    let project = row.get::<String>(0)?;
    let entry_key = row.get::<String>(1)?;
    let name = row.get::<String>(2)?;
    let metadata_json = row.get::<Option<String>>(3)?.unwrap_or_else(|| "[]".into());
    let metadata = serde_json::from_str(&metadata_json).unwrap_or_default();
    let access_count = row.get::<Option<i64>>(4)?.unwrap_or(0);
    let created_at = row.get::<Option<String>>(5)?;
    let updated_at = row.get::<Option<String>>(6)?;
    let created_unix = created_at
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        .map(|d| d.timestamp());
    let updated_unix = updated_at
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        .map(|d| d.timestamp());
    Ok(Citation {
        id: Some(entry_key.clone()),
        project,
        entry_key,
        name,
        metadata,
        access_count,
        created_unix,
        updated_unix,
    })
}
/// Upsert a citation row keyed by (`project`, `entry_key`). Refreshes `name` +
/// `metadata` and bumps `updated_at` on an existing row; creates one otherwise.
///
/// # Errors
/// Returns `Error::Turso` on query failure, `Error::Migration` when the CREATE
/// yields no id row, `Error::Validation` when a metadata URL is empty (the
/// evidence-gate invariant: a citation must cite a real source).
pub async fn upsert_citation(db: &TursoStore, c: &UpsertCitation<'_>) -> Result<String> {
    if c.metadata.iter().any(|m| m.url.trim().is_empty()) {
        return Err(crate::error::Error::Validation(
            "citation metadata URL must not be empty".to_owned(),
        ));
    }
    let metadata_json = serde_json::to_string(&c.metadata)?;
    let now = crate::turso::now_rfc3339();
    let find = "SELECT entry_key FROM citations \
                WHERE project = $project AND entry_key = $key LIMIT 1";
    let mut response = db
        .exec_params(
            find,
            libsql::params![c.project.clone(), c.entry_key.to_owned()],
        )
        .await?;
    let existing: Option<String> = match response.next().await? {
        Some(row) => Some(row.get::<String>(0)?),
        None => None,
    };
    if let Some(id) = existing {
        db.exec_params(
            "UPDATE citations SET name = $name, metadata = $meta, updated_at = $now \
             WHERE project = $project AND entry_key = $key",
            libsql::params![
                c.name.to_owned(),
                metadata_json,
                now,
                c.project.clone(),
                c.entry_key.to_owned()
            ],
        )
        .await?;
        Ok(id)
    } else {
        db.exec_params(
            "INSERT INTO citations (project, entry_key, name, slug, url, created_at, updated_at) \
             VALUES ($project, $key, $name, $slug, $url, $now, $now)",
            libsql::params![
                c.project.clone(),
                c.entry_key.to_owned(),
                c.name.to_owned(),
                c.metadata
                    .first()
                    .map(|m| m.slug.clone())
                    .unwrap_or_default(),
                c.metadata
                    .first()
                    .map(|m| m.url.clone())
                    .unwrap_or_default(),
                now.clone(),
                now
            ],
        )
        .await?;
        Ok(c.entry_key.to_owned())
    }
}
/// Fetch one citation by (`project`, `entry_key`), bumping `access_count`.
///
/// # Errors
/// Propagates `Error::Turso` from the SELECT/UPDATE.
pub async fn get_citation(
    db: &TursoStore,
    project: &str,
    entry_key: &str,
) -> Result<Option<Citation>> {
    let q = format!(
        "UPDATE citations SET access_count = access_count + 1 \
         WHERE project = $project AND entry_key = $key \
         RETURNING {COLS}"
    );
    let mut response = db
        .exec_params(q, libsql::params![project.to_owned(), entry_key.to_owned()])
        .await?;
    match response.next().await? {
        Some(row) => Ok(Some(citation_from_row(&row)?)),
        None => Ok(None),
    }
}
/// List every citation row for `project`, newest-updated first.
///
/// # Errors
/// Propagates `Error::Turso` from the SELECT.
pub async fn list_citations_by_project(db: &TursoStore, project: &str) -> Result<Vec<Citation>> {
    let q = format!(
        "SELECT {COLS} FROM citations \
         WHERE project = $project ORDER BY updated_at DESC"
    );
    let mut rows = db
        .exec_params(q, libsql::params![project.to_owned()])
        .await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push(citation_from_row(&row)?);
    }
    Ok(out)
}
#[cfg(test)]
#[path = "citation_test.rs"]
mod tests;
