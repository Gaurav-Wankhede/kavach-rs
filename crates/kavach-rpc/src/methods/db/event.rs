// ALGO: Append event + optional project resolution
// TIME: O(1) avg for event, O(n) worst for project_find_by_path
//! db.event RPC method — append audit event to log.

use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct EventParams {
    pub event_type: String,
    pub payload: Option<String>,
    #[serde(default)]
    pub work_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct EventResult {
    pub success: bool,
    pub id: Option<String>,
    pub error: Option<String>,
}

/// Append an event row to the audit log.
/// Project is resolved from the caller's `work_dir`.
///
/// # Errors
/// Returns an RPC `ErrorObjectOwned` when the database event append fails.
pub async fn event(ctx: &AppState, params: EventParams) -> Result<EventResult, ErrorObjectOwned> {
    let project_id = if params.work_dir.is_empty() {
        None
    } else {
        match kavach_surreal::project_find_by_path(&ctx.db, &params.work_dir).await {
            Ok(Some(p)) => p.id,
            Ok(None) | Err(_) => None,
        }
    };
    match kavach_surreal::append_event(
        &ctx.db,
        &params.event_type,
        "kavach-cli",
        project_id,
        params.payload.as_deref(),
    )
    .await
    {
        Ok(id) => Ok(EventResult {
            success: true,
            id: Some(format!("{id:?}")),
            error: None,
        }),
        Err(e) => Ok(EventResult {
            success: false,
            id: None,
            error: Some(e.to_string()),
        }),
    }
}
