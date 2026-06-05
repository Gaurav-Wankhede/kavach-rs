// ALGO: Update + validate
// TIME: O(1) avg | SPACE: O(1)
//! `db.status_update` RPC method.

use super::util::resolve_project_id;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_types::MemoryStatus;
use serde::{Deserialize, Serialize};
use std::str::FromStr as _;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct StatusUpdateParams {
    pub project: String,
    pub category: String,
    pub key: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct StatusUpdateResult {
    pub success: bool,
    pub error: Option<String>,
}

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when status validation fails.
pub async fn status_update(
    ctx: &AppState,
    params: StatusUpdateParams,
) -> Result<StatusUpdateResult, ErrorObjectOwned> {
    if MemoryStatus::from_str(&params.status).is_err() {
        return Ok(StatusUpdateResult {
            success: false,
            error: Some(format!(
                "invalid status '{}'; allowed: {}",
                params.status,
                MemoryStatus::allowed_list()
            )),
        });
    }
    let pid = resolve_project_id(&ctx.db, &params.project).await?;
    let result =
        kavach_surreal::update_status(&ctx.db, &params.category, &pid, &params.key, &params.status)
            .await;

    match result {
        // FAIL CLOSED: update_status returns the matched-row count. A bare
        // UPDATE ... WHERE on an absent key affects 0 rows yet returns Ok —
        // reporting that as success let a phantom key read back as a real
        // transition (silent-success class; the loop's only honest progress
        // signal is a card move, so a no-op move must NOT count). Mirror the
        // `write --update-key` contract: refuse an unknown key.
        Ok(0) => Ok(StatusUpdateResult {
            success: false,
            error: Some(format!(
                "no '{}' row with key '{}' in project '{}' — nothing updated. \
                 Run `kavach db query --project {} --category {}` to list valid keys.",
                params.category, params.key, params.project, params.project, params.category
            )),
        }),
        Ok(_) => Ok(StatusUpdateResult {
            success: true,
            error: None,
        }),
        Err(e) => Ok(StatusUpdateResult {
            success: false,
            error: Some(e.to_string()),
        }),
    }
}
