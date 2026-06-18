// TIME: O(n) registered rows | SPACE: O(1)
// YEAR: 2026 | SEARCHED: 2026-06
//! `db.find_project` / `db.find_part` RPC methods — locate by absolute path.
//!
//! Reads routed through the single-writer daemon.
//! SOURCE: <https://github.com/facebook/rocksdb/issues/1780>

use crate::error::{internal, invalid_params};
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct FindParams {
    pub abs_path: String,
}

/// Match result; `None` fields mean no row matched the path.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct FindResult {
    pub label: Option<String>,
    pub detail: Option<String>,
}

/// # Errors
/// Returns an RPC error when the path is not absolute or the read fails.
pub async fn find_project(
    ctx: &AppState,
    params: FindParams,
) -> Result<FindResult, ErrorObjectOwned> {
    require_absolute(&params.abs_path)?;
    let hit = kavach_surreal::project_find_by_path(&ctx.db, &params.abs_path)
        .await
        .map_err(|e| internal(e.to_string()))?;
    Ok(hit.map_or(
        FindResult {
            label: None,
            detail: None,
        },
        |p| FindResult {
            label: Some(p.slug),
            detail: Some(p.workdir.unwrap_or_else(|| "?".to_owned())),
        },
    ))
}

/// # Errors
/// Returns an RPC error when the path is not absolute or the read fails.
pub async fn find_part(ctx: &AppState, params: FindParams) -> Result<FindResult, ErrorObjectOwned> {
    require_absolute(&params.abs_path)?;
    let hit = kavach_surreal::part_find_by_path(&ctx.db, &params.abs_path)
        .await
        .map_err(|e| internal(e.to_string()))?;
    Ok(hit.map_or(
        FindResult {
            label: None,
            detail: None,
        },
        |p| FindResult {
            label: Some(p.part_name),
            detail: Some(p.part_path),
        },
    ))
}

fn require_absolute(p: &str) -> Result<(), ErrorObjectOwned> {
    if p.starts_with('/') {
        Ok(())
    } else {
        Err(invalid_params(format!("path must be absolute: {p}")))
    }
}
