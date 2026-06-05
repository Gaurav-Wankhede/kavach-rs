// ALGO: per-table archive-by-expiry scan
// PROBLEM_CLASS: retention-archive
// TIME: O(n) entries past expiry | SPACE: O(t) tables
// YEAR: 2026 | SEARCHED: 2026-06
//! `db.expire` RPC method — archive entries past their `expires_at`.
//!
//! A WRITE: routes through the single-writer daemon.
//! SOURCE: <https://github.com/facebook/rocksdb/issues/1780>

use crate::error::internal;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};

/// No request fields — `expire` scans all typed tables.
///
/// A unit struct (serializes to `null`) matches the typed-params call shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct ExpireParams;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct ExpireResult {
    pub archived_total: usize,
    pub per_table: Vec<(String, usize)>,
}

/// # Errors
/// Returns an RPC error when the archive sweep fails.
pub async fn expire(
    ctx: &AppState,
    _params: ExpireParams,
) -> Result<ExpireResult, ErrorObjectOwned> {
    let report = kavach_surreal::expire_stale(&ctx.db)
        .await
        .map_err(|e| internal(e.to_string()))?;
    Ok(ExpireResult {
        archived_total: report.archived_total,
        per_table: report.per_table,
    })
}
