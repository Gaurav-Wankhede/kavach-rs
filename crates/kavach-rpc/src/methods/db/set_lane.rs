// TIME: O(1) avg | SPACE: O(1)
//! `db.set_lane` RPC method — pin a roadmap card to a dispatch lane.

use super::util::resolve_project_id;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct SetLaneParams {
    pub project: String,
    pub category: String,
    pub key: String,
    /// `Some(name)` pins the card to that lane; `None` clears it back to the
    /// unlaned general backlog.
    #[serde(default)]
    pub lane: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct SetLaneResult {
    pub success: bool,
    pub id: Option<String>,
    pub error: Option<String>,
}

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when the database update fails.
pub async fn set_lane(
    ctx: &AppState,
    params: SetLaneParams,
) -> Result<SetLaneResult, ErrorObjectOwned> {
    let pid = resolve_project_id(&ctx.db, &params.project).await?;
    let result =
        kavach_surreal::set_lane(&ctx.db, &params.category, &pid, &params.key, params.lane).await;
    match result {
        Ok(id) => Ok(SetLaneResult {
            success: true,
            id: Some(format!("{id:?}")),
            error: None,
        }),
        Err(e) => Ok(SetLaneResult {
            success: false,
            id: None,
            error: Some(e.to_string()),
        }),
    }
}
