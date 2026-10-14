// split: intentional - cohesive rag_tree storage (struct + 2 read fns)
// SQL-backed rag_tree store. Mirrors kavach-db::rag_trees read API.
// sql-safe: explicit column list; bound params only; no string concat.
use crate::error::Result;
use crate::db::DbStore;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RagTreeRow {
    pub source: String,
    pub built_at: String,
    pub tree_json: Vec<u8>,
    pub source_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RagTreeLabel {
    pub source: String,
    pub built_at: String,
    pub source_hash: String,
}

/// Fetch a `rag_tree` row by source label.
///
/// # Errors
/// Propagates `Error::Database` from the SELECT.
pub async fn get(db: &DbStore, source: &str) -> Result<Option<RagTreeRow>> {
    let mut rows = db
        .exec_params(
            "SELECT source, built_at, tree_json, source_hash FROM rag_tree \
             WHERE source = $source LIMIT 1",
            libsql::params![source.to_owned()],
        )
        .await?;
    match rows.next().await? {
        Some(r) => Ok(Some(RagTreeRow {
            source: r.get::<String>(0)?,
            built_at: r.get::<String>(1)?,
            tree_json: r.get::<Vec<u8>>(2)?,
            source_hash: r.get::<String>(3)?,
        })),
        None => Ok(None),
    }
}

/// List all `rag_tree` rows (label-only projection).
///
/// # Errors
/// Propagates `Error::Database` from the SELECT.
pub async fn list(db: &DbStore) -> Result<Vec<RagTreeLabel>> {
    let mut rows = db
        .exec_params("SELECT source, built_at, source_hash FROM rag_tree", ())
        .await?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await? {
        out.push(RagTreeLabel {
            source: r.get::<String>(0)?,
            built_at: r.get::<String>(1)?,
            source_hash: r.get::<String>(2)?,
        });
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RagTreeRefreshable {
    pub source: String,
    pub source_dir: String,
}

/// List all `rag_tree` labels with non-empty `source_dir`, for stale-refresh.
///
/// # Errors
/// Propagates `Error::Database` from the SELECT.
pub async fn list_refreshable(db: &DbStore) -> Result<Vec<RagTreeRefreshable>> {
    let mut rows = db
        .exec_params(
            "SELECT source, source_dir FROM rag_tree WHERE source_dir != ''",
            (),
        )
        .await?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await? {
        out.push(RagTreeRefreshable {
            source: r.get::<String>(0)?,
            source_dir: r.get::<String>(1)?,
        });
    }
    Ok(out)
}

/// Upsert a `rag_tree` row by `source` (label).
///
/// Stores the NDJSON tree blob, the BLAKE3 source hash, and the originating
/// source directory so `refresh-if-stale` can detect drift. Keyed UPSERT on the
/// `source` primary key.
///
/// # Errors
/// Propagates `Error::Database` from the UPSERT.
pub async fn upsert_with_dir(
    db: &DbStore,
    source: &str,
    built_at: &str,
    tree_json: &[u8],
    source_hash: &str,
    source_dir: &str,
) -> Result<()> {
    let now = crate::db::now_rfc3339();
    db.exec_params(
        "INSERT INTO rag_tree (source, built_at, tree_json, source_hash, source_dir, updated_at) \
         VALUES ($source, $built_at, $tree_json, $source_hash, $source_dir, $now) \
         ON CONFLICT(source) DO UPDATE SET \
             built_at = excluded.built_at, tree_json = excluded.tree_json, \
             source_hash = excluded.source_hash, source_dir = excluded.source_dir, \
             updated_at = excluded.updated_at",
        libsql::params![
            source.to_owned(),
            built_at.to_owned(),
            tree_json.to_vec(),
            source_hash.to_owned(),
            source_dir.to_owned(),
            now
        ],
    )
    .await?;
    Ok(())
}
