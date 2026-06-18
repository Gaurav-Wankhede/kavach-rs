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

pub use crate::graph::traverse_with_citations as traverse;

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
        db.query(
            "UPDATE $id SET name = $name, metadata = $meta, updated_at = time::now()",
        )
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
mod tests;
