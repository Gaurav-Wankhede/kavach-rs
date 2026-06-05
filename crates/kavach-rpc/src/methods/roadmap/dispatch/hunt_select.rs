use super::super::readiness::{deps_satisfied, is_runnable_status};
use super::super::types::{NextOpenTaskParams, NextTaskResult};
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;

const TABLE_ROADMAP: &str = "roadmap";
const HUNT_KEY_PREFIX: &str = "hunt.";

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when the database query fails.
pub async fn next_open_hunt(
    state: &AppState,
    params: NextOpenTaskParams,
) -> Result<Option<NextTaskResult>, ErrorObjectOwned> {
    let Some(project) = kavach_surreal::project_get_by_slug(&state.db, &params.project)
        .await
        .map_err(surreal_to_rpc)?
    else {
        return Ok(None);
    };
    let Some(project_id) = project.id else {
        return Ok(None);
    };
    let entries = kavach_surreal::list_by_project(&state.db, TABLE_ROADMAP, &project_id)
        .await
        .map_err(surreal_to_rpc)?;
    let dep_pool = kavach_surreal::list_all_by_table(&state.db, TABLE_ROADMAP)
        .await
        .map_err(surreal_to_rpc)?;
    let hunt = entries
        .iter()
        .find(|e| {
            e.entry_key.starts_with(HUNT_KEY_PREFIX)
                && is_runnable_status(e.entry_status_str())
                && deps_satisfied(e, &dep_pool)
        })
        .map(|e| NextTaskResult {
            key: e.entry_key.clone(),
            title: e.title.clone(),
            status: e.entry_status_str().to_owned(),
        });
    Ok(hunt)
}
