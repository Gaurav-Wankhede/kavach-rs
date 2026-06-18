// TIME: O(1) avg | SPACE: O(1)
//! `db.set_priority` RPC method.

use super::util::resolve_project_id;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_types::Priority;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct SetPriorityParams {
    pub project: String,
    pub category: String,
    pub key: String,
    #[serde(default)]
    pub priority: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct SetPriorityResult {
    pub success: bool,
    pub id: Option<String>,
    pub error: Option<String>,
}

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when the database update fails.
pub async fn set_priority(
    ctx: &AppState,
    params: SetPriorityParams,
) -> Result<SetPriorityResult, ErrorObjectOwned> {
    let pid = resolve_project_id(&ctx.db, &params.project).await?;
    let priority = params.priority.map(Priority::new);
    let result =
        kavach_surreal::set_priority(&ctx.db, &params.category, &pid, &params.key, priority).await;
    match result {
        Ok(id) => Ok(SetPriorityResult {
            success: true,
            id: Some(format!("{id:?}")),
            error: None,
        }),
        Err(e) => Ok(SetPriorityResult {
            success: false,
            id: None,
            error: Some(e.to_string()),
        }),
    }
}
