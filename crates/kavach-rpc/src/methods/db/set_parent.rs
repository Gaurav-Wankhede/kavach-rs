// TIME: O(1) | SPACE: O(1)
// YEAR: 2026 | SEARCHED: 2026-06
//! `db.set_parent` RPC method — link/detach a project's parent.
//!
//! A WRITE: routes through the single-writer daemon so the CLI never opens a
//! second `RocksDB` handle. SOURCE: <https://github.com/facebook/rocksdb/issues/1780>

use crate::error::internal;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct SetParentParams {
    pub child: String,
    pub parent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct SetParentResult {
    pub message: String,
}

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when the write fails.
pub async fn set_parent(
    ctx: &AppState,
    params: SetParentParams,
) -> Result<SetParentResult, ErrorObjectOwned> {
    kavach_surreal::project_set_parent(&ctx.db, &params.child, params.parent.as_deref())
        .await
        .map_err(|e| internal(e.to_string()))?;
    let message = params.parent.map_or_else(
        || format!("detached {} to top-level", params.child),
        |p| format!("linked {} -> parent {p}", params.child),
    );
    Ok(SetParentResult { message })
}
