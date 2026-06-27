// SurrealDB-backed citation store: official-docs context rows keyed by
// (project, entry_key) via the UNIQUE idx_citation_project_key. Mirrors the
// gate_patterns CRUD idiom — explicit column list, bound params, take(0) into a
// typed struct. Datetimes project through time::unix() to i64 (no chrono dep).
// sql-safe: static literals + .bind() only; no string concat.
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::{RecordId, SurrealValue};
pub use crate::graph::{citations_cited_by, traverse_with_citations as traverse};
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
pub async fn merge_node_into_citation(
    db: &Surreal<Db>,
    node: &RecordId,
    citation: &RecordId,
) -> Result<()> {
    let mut response = db
        .query("SELECT VALUE id FROM cite WHERE in = $node AND out = $cit")
        .bind(("node", node.clone()))
        .bind(("cit", citation.clone()))
        .await?;
    // The `cite` edge table is absent until the first RELATE creates it; a
    // missing table is the "no edge yet" case, not a failure.
    let existing: Vec<RecordId> = match response.take(0) {
        Ok(rows) => rows,
        Err(e) if crate::error::is_missing_table_error(&e) => Vec::new(),
        Err(e) => return Err(e.into()),
    };
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
/// Propagates `Error::Surreal` from the traversal.
pub async fn citations_for_nodes(db: &Surreal<Db>, nodes: &[RecordId]) -> Result<Vec<RecordId>> {
    if nodes.is_empty() {
        return Ok(Vec::new());
    }
    let mut response = db
        .query("SELECT VALUE ->cite->citation FROM $nodes")
        .bind(("nodes", nodes.to_vec()))
        .await?;
    let per_node: Vec<Vec<RecordId>> = match response.take(0) {
        Ok(rows) => rows,
        Err(e) if crate::error::is_missing_table_error(&e) => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let mut out: Vec<RecordId> = Vec::new();
    for cit in per_node.into_iter().flatten() {
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
/// Propagates `Error::Surreal` from the UPDATE.
pub async fn reward_citation_edges(
    db: &Surreal<Db>,
    citation: &RecordId,
    delta: f64,
) -> Result<usize> {
    let mut response = db
        .query("UPDATE cite SET weight += $delta WHERE out = $cit RETURN id")
        .bind(("cit", citation.clone()))
        .bind(("delta", delta))
        .await?;
    let updated: Vec<RecordId> = match response.take(0) {
        Ok(rows) => rows,
        Err(e) if crate::error::is_missing_table_error(&e) => Vec::new(),
        Err(e) => return Err(e.into()),
    };
    Ok(updated.len())
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
const COLS: &str = "id, project, entry_key, name, metadata, access_count, \
                    time::unix(created_at) AS created_unix, \
                    time::unix(updated_at) AS updated_unix";
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
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
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[non_exhaustive]
pub struct Citation {
    pub id: Option<RecordId>,
    pub project: RecordId,
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
#[derive(surrealdb_types::SurrealValue)]
struct IdRow {
    id: RecordId,
}
#[derive(Debug)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate struct-literal DTO (kavach-rpc); non_exhaustive => E0639"
)]
pub struct UpsertCitation<'a> {
    pub project: RecordId,
    pub entry_key: &'a str,
    pub name: &'a str,
    pub metadata: Vec<CitationMeta>,
}
/// Upsert a citation row keyed by (`project`, `entry_key`). Refreshes `name` +
/// `metadata` and bumps `updated_at` on an existing row; creates one otherwise.
///
/// # Errors
/// Returns `Error::Surreal` on query failure, `Error::Migration` when the CREATE
/// yields no id row.
pub async fn upsert_citation(db: &Surreal<Db>, c: &UpsertCitation<'_>) -> Result<RecordId> {
    let find = "SELECT id FROM citation \
                WHERE project = $project AND entry_key = $key LIMIT 1";
    let mut response = db
        .query(find)
        .bind(("project", c.project.clone()))
        .bind(("key", c.entry_key.to_owned()))
        .await?;
    let existing: Option<IdRow> = response.take(0)?;
    if let Some(IdRow { id }) = existing {
        db.query("UPDATE $id SET name = $name, metadata = $meta, updated_at = time::now()")
            .bind(("id", id.clone()))
            .bind(("name", c.name.to_owned()))
            .bind(("meta", c.metadata.clone()))
            .await?
            .check()?;
        Ok(id)
    } else {
        let mut resp = db
            .query(
                "CREATE citation SET project = $project, entry_key = $key, \
                 name = $name, metadata = $meta RETURN id",
            )
            .bind(("project", c.project.clone()))
            .bind(("key", c.entry_key.to_owned()))
            .bind(("name", c.name.to_owned()))
            .bind(("meta", c.metadata.clone()))
            .await?;
        let row: Option<IdRow> = resp.take(0)?;
        row.map(|r| r.id)
            .ok_or_else(|| Error::Migration("citation create returned no id".into()))
    }
}
/// Fetch one citation by (`project`, `entry_key`), bumping `access_count`.
///
/// # Errors
/// Propagates `Error::Surreal` from the SELECT/UPDATE.
pub async fn get_citation(
    db: &Surreal<Db>,
    project: &RecordId,
    entry_key: &str,
) -> Result<Option<Citation>> {
    let q = format!(
        "UPDATE citation SET access_count += 1 \
         WHERE project = $project AND entry_key = $key \
         RETURN {COLS}"
    );
    let mut response = db
        .query(q)
        .bind(("project", project.clone()))
        .bind(("key", entry_key.to_owned()))
        .await?;
    match response.take::<Vec<Citation>>(0) {
        Ok(mut rows) => Ok(rows.pop()),
        Err(e) if crate::error::is_missing_table_error(&e) => Ok(None),
        Err(e) => Err(e.into()),
    }
}
/// List every citation row for `project`, newest-updated first.
///
/// # Errors
/// Propagates `Error::Surreal` from the SELECT.
pub async fn list_citations_by_project(
    db: &Surreal<Db>,
    project: &RecordId,
) -> Result<Vec<Citation>> {
    let q = format!(
        "SELECT {COLS} FROM citation \
         WHERE project = $project ORDER BY updated_unix DESC"
    );
    let mut response = db.query(q).bind(("project", project.clone())).await?;
    match response.take::<Vec<Citation>>(0) {
        Ok(rows) => Ok(rows),
        Err(e) if crate::error::is_missing_table_error(&e) => Ok(Vec::new()),
        Err(e) => Err(e.into()),
    }
}
#[cfg(test)]
#[path = "citation_test.rs"]
#[cfg(test)]
#[path = "citation_test.rs"]
mod tests;
