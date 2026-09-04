// split: intentional - cohesive rag_tree storage (struct + 2 read fns)
// SurrealDB-backed rag_tree store. Mirrors kavach-db::rag_trees read API.
// sql-safe: explicit column list; bound params only; no string concat.
use crate::error::Result;
use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::SurrealValue;

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[non_exhaustive]
pub struct RagTreeRow {
    pub source: String,
    pub built_at: String,
    pub tree_json: surrealdb_types::Bytes,
    pub source_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[non_exhaustive]
pub struct RagTreeLabel {
    pub source: String,
    pub built_at: String,
    pub source_hash: String,
}

/// Fetch a `rag_tree` row by source label.
///
/// # Errors
/// Propagates `Error::Surreal` from the SELECT.
pub async fn get(db: &Surreal<Db>, source: &str) -> Result<Option<RagTreeRow>> {
    let q = "SELECT source, built_at, tree_json, source_hash FROM rag_tree \
             WHERE source = $source LIMIT 1";
    let mut response = db.query(q).bind(("source", source.to_owned())).await?;
    match response.take::<Option<RagTreeRow>>(0) {
        Ok(row) => Ok(row),
        Err(e) if crate::error::is_missing_table_error(&e) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// List all `rag_tree` rows (label-only projection).
///
/// # Errors
/// Propagates `Error::Surreal` from the SELECT.
pub async fn list(db: &Surreal<Db>) -> Result<Vec<RagTreeLabel>> {
    let q = "SELECT source, built_at, source_hash FROM rag_tree";
    let mut response = db.query(q).await?;
    let rows: Vec<RagTreeLabel> = response.take(0)?;
    Ok(rows)
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[non_exhaustive]
pub struct RagTreeRefreshable {
    pub source: String,
    pub source_dir: String,
}

/// List all `rag_tree` labels with non-empty `source_dir`, for stale-refresh.
///
/// # Errors
/// Propagates `Error::Surreal` from the SELECT.
pub async fn list_refreshable(db: &Surreal<Db>) -> Result<Vec<RagTreeRefreshable>> {
    let q = "SELECT source, source_dir FROM rag_tree WHERE source_dir != ''";
    let mut response = db.query(q).await?;
    let rows: Vec<RagTreeRefreshable> = response.take(0)?;
    Ok(rows)
}

/// Upsert a `rag_tree` row by `source` (label).
///
/// Mirrors the prior `SQLite` shape: stores the NDJSON tree blob, the
/// BLAKE3 source hash, and the originating source directory so
/// `refresh-if-stale` can detect drift. Uses the UNIQUE index
/// `idx_rag_tree_source` for O(1) lookup.
///
/// # Errors
/// Propagates `Error::Surreal` from the UPSERT.
pub async fn upsert_with_dir(
    db: &Surreal<Db>,
    source: &str,
    built_at: &str,
    tree_json: &[u8],
    source_hash: &str,
    source_dir: &str,
) -> Result<()> {
    let q = "UPSERT type::record('rag_tree', $source) SET \
                source = $source, \
                built_at = $built_at, \
                tree_json = $tree_json, \
                source_hash = $source_hash, \
                source_dir = $source_dir, \
                updated_at = time::now()";
    db.query(q)
        .bind(("source", source.to_owned()))
        .bind(("built_at", built_at.to_owned()))
        .bind((
            "tree_json",
            surrealdb_types::Bytes::from(tree_json.to_vec()),
        ))
        .bind(("source_hash", source_hash.to_owned()))
        .bind(("source_dir", source_dir.to_owned()))
        .await?;
    Ok(())
}
