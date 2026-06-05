// `db.set_harness` — write the AI-chosen dynamic-workflow pattern + compiled
// `workflow.js` path onto a roadmap card. Mirrors `db::set_priority`.
use super::resolve::project_id;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at the db.* handler boundary; non_exhaustive => E0639 on construct"
)]
pub struct SetHarnessParams {
    pub project: String,
    pub key: String,
    /// `Some(pattern)` sets the harness; `None` clears it (ordinary dispatch).
    #[serde(default)]
    pub harness: Option<String>,
    /// Path to the compiled `workflow.js` the stop gate runs.
    #[serde(default)]
    pub workflow_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at the db.* handler boundary; non_exhaustive => E0639 on construct"
)]
pub struct SetHarnessResult {
    pub success: bool,
    pub id: Option<String>,
    pub error: Option<String>,
}

/// Set (or clear) the harness pattern + `workflow_path` on a roadmap card.
///
/// # Errors
/// Returns an RPC `ErrorObjectOwned` only when project resolution fails; a
/// failed UPDATE is reported inside `SetHarnessResult::error`.
pub async fn set_harness(
    state: &AppState,
    params: SetHarnessParams,
) -> Result<SetHarnessResult, ErrorObjectOwned> {
    let pid = project_id(state, &params.project).await?;
    let result = kavach_surreal::set_harness(
        &state.db,
        &pid,
        &params.key,
        params.harness.as_deref(),
        params.workflow_path.as_deref(),
    )
    .await;
    match result {
        Ok(id) => Ok(SetHarnessResult {
            success: true,
            id: Some(format!("{id:?}")),
            error: None,
        }),
        Err(e) => Ok(SetHarnessResult {
            success: false,
            id: None,
            error: Some(e.to_string()),
        }),
    }
}
