// TIME: O(1) | SPACE: O(1)
// YEAR: 2026 | SEARCHED: 2026-06
//! `db.register` RPC method — register or update a project.
//!
//! A WRITE: routes through the single-writer daemon.
//! SOURCE: <https://github.com/facebook/rocksdb/issues/1780>

use crate::error::{internal, invalid_params};
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct RegisterParams {
    pub slug: String,
    pub abs_path: String,
    pub stack: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct RegisterResult {
    pub message: String,
}

/// # Errors
/// Returns an RPC error when the path is not absolute or the write fails.
pub async fn register(
    ctx: &AppState,
    params: RegisterParams,
) -> Result<RegisterResult, ErrorObjectOwned> {
    if !params.abs_path.starts_with('/') {
        return Err(invalid_params(format!(
            "path must be absolute: {}",
            params.abs_path
        )));
    }
    let id = kavach_surreal::project_register(
        &ctx.db,
        &params.slug,
        &params.slug,
        &params.abs_path,
        params.stack.as_deref(),
    )
    .await
    .map_err(|e| internal(e.to_string()))?;
    Ok(RegisterResult {
        message: format!(
            "registered project: {} (id={id:?}) at {}",
            params.slug, params.abs_path
        ),
    })
}
