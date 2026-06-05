// `db.get_harness` — read the (harness, workflow_path) link off a roadmap card.
// L3 stop gate calls this to decide whether to auto-run a compiled workflow.
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
pub struct GetHarnessParams {
    pub project: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at the db.* handler boundary; non_exhaustive => E0639 on construct"
)]
pub struct GetHarnessResult {
    /// `false` when the (project, key) roadmap row is absent.
    pub found: bool,
    pub harness: Option<String>,
    pub workflow_path: Option<String>,
}

/// Read the harness link for one roadmap card.
///
/// # Errors
/// Returns an RPC `ErrorObjectOwned` when project resolution or the SELECT fails.
pub async fn get_harness(
    state: &AppState,
    params: GetHarnessParams,
) -> Result<GetHarnessResult, ErrorObjectOwned> {
    let pid = project_id(state, &params.project).await?;
    let link = kavach_surreal::harness_link::get_harness_link(&state.db, &pid, &params.key)
        .await
        .map_err(|e| internal(e.to_string()))?;
    match link {
        Some(l) => Ok(GetHarnessResult {
            found: true,
            harness: l.harness,
            workflow_path: l.workflow_path,
        }),
        None => Ok(GetHarnessResult {
            found: false,
            harness: None,
            workflow_path: None,
        }),
    }
}
