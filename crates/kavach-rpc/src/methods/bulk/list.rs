// bulk.sweep_list_active — list active manifests for a project. Used by
// `kavach bulk status` + stop-gate (refuses clean stop while sweep in-flight).
use crate::error::surreal_to_rpc;
use crate::methods::bulk::get::GetResult;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::bulk_manifest::list_active;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ListActiveParams {
    pub project: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ListActiveResult {
    pub manifests: Vec<GetResult>,
}

/// Lists active manifests for a project.
///
/// # Errors
///
/// Returns an error if the `SurrealDB` query fails or the project does not exist.
pub async fn list_active_rpc(
    state: &AppState,
    p: ListActiveParams,
) -> Result<ListActiveResult, ErrorObjectOwned> {
    let rows = list_active(&state.db, &p.project)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(ListActiveResult {
        manifests: rows.into_iter().map(Into::into).collect(),
    })
}
