// ALGO: Time-based filter + update
// TIME: O(n) | SPACE: O(n)
//! db.archive RPC method — archive stale entries.

use crate::error::internal;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};

const DEFAULT_FLOOR_DAYS: i64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct ArchiveParams {
    #[serde(default = "default_floor_days_const")]
    pub floor_days: i64,
    #[serde(default)]
    pub dry_run: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct ArchiveResult {
    pub success: bool,
    pub archived_count: usize,
    pub dry_run: bool,
    pub error: Option<String>,
}

const fn default_floor_days_const() -> i64 {
    DEFAULT_FLOOR_DAYS
}

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when the database archive operation fails.
pub async fn archive(
    ctx: &AppState,
    params: ArchiveParams,
) -> Result<ArchiveResult, ErrorObjectOwned> {
    let dry_run = params.dry_run.unwrap_or_default();
    let report = kavach_surreal::archive_irrelevant(&ctx.db, params.floor_days, dry_run)
        .await
        .map_err(|e| internal(e.to_string()))?;

    Ok(ArchiveResult {
        success: true,
        archived_count: report.archived,
        dry_run,
        error: None,
    })
}
