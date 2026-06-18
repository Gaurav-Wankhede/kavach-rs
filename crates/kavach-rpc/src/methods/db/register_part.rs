// TIME: O(1) | SPACE: O(1)
// YEAR: 2026 | SEARCHED: 2026-06
//! `db.register_part` RPC method — register or update a project part.
//!
//! A WRITE: routes through the single-writer daemon.
//! SOURCE: <https://github.com/facebook/rocksdb/issues/1780>

use super::util::resolve_project_id;
use crate::error::{internal, invalid_params};
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct RegisterPartParams {
    pub project: String,
    pub name: String,
    pub abs_path: String,
    pub part_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct RegisterPartResult {
    pub message: String,
}

/// # Errors
/// Returns an RPC error when the path is not absolute, the project is unknown,
/// or the write fails.
pub async fn register_part(
    ctx: &AppState,
    params: RegisterPartParams,
) -> Result<RegisterPartResult, ErrorObjectOwned> {
    if !params.abs_path.starts_with('/') {
        return Err(invalid_params(format!(
            "path must be absolute: {}",
            params.abs_path
        )));
    }
    let pid = resolve_project_id(&ctx.db, &params.project).await?;
    let id = kavach_surreal::part_upsert(
        &ctx.db,
        &pid,
        &params.name,
        &params.abs_path,
        &params.part_type,
        None,
        None,
    )
    .await
    .map_err(|e| internal(e.to_string()))?;
    Ok(RegisterPartResult {
        message: format!(
            "registered part: {} ({}) id={id:?} at {}",
            params.name, params.part_type, params.abs_path
        ),
    })
}
