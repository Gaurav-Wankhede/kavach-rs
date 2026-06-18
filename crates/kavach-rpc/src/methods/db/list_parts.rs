// TIME: O(p) parts for the project | SPACE: O(p)
// YEAR: 2026 | SEARCHED: 2026-06
//! `db.list_parts` RPC method — registry read of a project's parts.
//!
//! Companion to `db.list_projects`; routes `kavach db list-parts <slug>` through
//! the single-writer daemon so the CLI never opens a second `RocksDB` handle
//! while the daemon holds the `fcntl` lock.
//! SOURCE: <https://github.com/facebook/rocksdb/issues/1780>

use super::util::resolve_project_id;
use crate::error::internal;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct ListPartsParams {
    pub project: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct ListPartsResult {
    pub parts: Vec<PartRow>,
}

/// One part, flattened to display strings so RPC and the direct fallback render
/// byte-identical output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct PartRow {
    pub part_name: String,
    pub part_type: String,
    pub part_path: String,
    pub stack: Option<String>,
}

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when the project is unknown or the read fails.
pub async fn list_parts(
    ctx: &AppState,
    params: ListPartsParams,
) -> Result<ListPartsResult, ErrorObjectOwned> {
    let pid = resolve_project_id(&ctx.db, &params.project).await?;
    let parts = kavach_surreal::parts_list_by_project(&ctx.db, &pid)
        .await
        .map_err(|e| internal(e.to_string()))?
        .into_iter()
        .map(|p| PartRow {
            part_name: p.part_name,
            part_type: p.part_type,
            part_path: p.part_path,
            stack: p.stack,
        })
        .collect();
    Ok(ListPartsResult { parts })
}
