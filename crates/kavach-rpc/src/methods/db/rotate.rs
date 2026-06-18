// TIME: O(n) matched events | SPACE: O(1)
// YEAR: 2026 | SEARCHED: 2026-06
//! `db.rotate` RPC method — delete events older than N days.
//!
//! A destructive WRITE: routes through the single-writer daemon.
//! SOURCE: <https://github.com/facebook/rocksdb/issues/1780>

use crate::error::{internal, invalid_params};
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct RotateParams {
    pub days: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct RotateResult {
    pub rotated: usize,
    pub days: i64,
}

/// # Errors
/// Returns an RPC error when `days <= 0` or the delete fails.
pub async fn rotate(
    ctx: &AppState,
    params: RotateParams,
) -> Result<RotateResult, ErrorObjectOwned> {
    if params.days <= 0 {
        return Err(invalid_params("days must be positive"));
    }
    let rotated = kavach_surreal::rotate_events(&ctx.db, params.days)
        .await
        .map_err(|e| internal(e.to_string()))?;
    Ok(RotateResult {
        rotated,
        days: params.days,
    })
}
