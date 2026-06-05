// `db.latest_goal_attempt` — read the oracle's most recent harness verdict so
// the stop gate can decide pass / retry / escalate.
use super::resolve::project_id;
use crate::error::internal;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at the db.* handler boundary; non_exhaustive => E0639 on construct"
)]
pub struct LatestAttemptParams {
    pub project: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at the db.* handler boundary; non_exhaustive => E0639 on construct"
)]
pub struct LatestAttemptResult {
    /// `false` when no `goal_loop_attempt` event exists yet for the project.
    pub found: bool,
    /// Raw oracle payload JSON (verdict, attempt count, exit code).
    pub payload: Option<serde_json::Value>,
}

/// Read the latest `goal_loop_attempt` verdict for a project.
///
/// # Errors
/// Returns an RPC `ErrorObjectOwned` when project resolution or the SELECT fails.
pub async fn latest_goal_attempt(
    state: &AppState,
    params: LatestAttemptParams,
) -> Result<LatestAttemptResult, ErrorObjectOwned> {
    let pid = project_id(state, &params.project).await?;
    let row = kavach_surreal::latest_goal_attempt(&state.db, &pid)
        .await
        .map_err(|e| internal(e.to_string()))?;
    match row {
        Some(a) => Ok(LatestAttemptResult {
            found: true,
            payload: a.payload.and_then(|v| serde_json::to_value(v).ok()),
        }),
        None => Ok(LatestAttemptResult {
            found: false,
            payload: None,
        }),
    }
}
