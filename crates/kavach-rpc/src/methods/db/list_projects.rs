// TIME: O(n log n) sort by slug | SPACE: O(n)
// YEAR: 2026 | SEARCHED: 2026-06
//! `db.list_projects` RPC method — registry read of all projects.
//!
//! Routes the CLI `kavach db list` (projects view) through the single-writer
//! daemon instead of a second direct `RocksDB` handle, closing the single-writer
//! invariant for this read path. The daemon owns the only `RocksDB` `fcntl` lock;
//! a CLI direct open would race it (`LOCK: Resource temporarily unavailable`).
//! SOURCE: <https://github.com/facebook/rocksdb/issues/1780>

use crate::error::internal;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};

/// No request fields — `list_projects` reads the entire registry.
///
/// A unit struct (serializes to `null`) keeps the typed-params call shape
/// uniform with the other db methods while satisfying
/// `clippy::empty_structs_with_brackets`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct ListProjectsParams;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct ListProjectsResult {
    pub projects: Vec<ProjectRow>,
}

/// One project, flattened to display-ready strings so the CLI never re-derives
/// the same defaults two ways (RPC vs fallback build identical output).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct ProjectRow {
    pub slug: String,
    pub workdir: Option<String>,
    pub stack: Option<String>,
}

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when the underlying `SurrealDB` read fails.
pub async fn list_projects(
    ctx: &AppState,
    _params: ListProjectsParams,
) -> Result<ListProjectsResult, ErrorObjectOwned> {
    let projects = kavach_surreal::projects_list_all(&ctx.db)
        .await
        .map_err(|e| internal(e.to_string()))?
        .into_iter()
        .map(|p| ProjectRow {
            slug: p.slug,
            workdir: p.workdir,
            stack: p.stack,
        })
        .collect();
    Ok(ListProjectsResult { projects })
}
