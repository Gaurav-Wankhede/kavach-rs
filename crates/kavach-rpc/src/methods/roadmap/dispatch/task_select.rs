use super::super::readiness::{deps_satisfied, is_runnable_status};
use super::super::types::{NextOpenTaskParams, NextTaskResult};
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;

const TABLE_ROADMAP: &str = "roadmap";

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when the database query fails.
pub async fn next_open_task(
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
    let mut selected: Option<NextTaskResult> = None;
    for e in &entries {
        if !is_runnable_status(e.entry_status_str()) {
            continue;
        }
        if deps_satisfied(e, &dep_pool) {
            selected = Some(NextTaskResult {
                key: e.entry_key.clone(),
                title: e.title.clone(),
                status: e.entry_status_str().to_owned(),
            });
            break;
        }
    }
    Ok(selected)
}

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when the database query fails.
pub async fn ready_set(
    state: &AppState,
    params: NextOpenTaskParams,
) -> Result<Vec<NextTaskResult>, ErrorObjectOwned> {
    let Some(project) = kavach_surreal::project_get_by_slug(&state.db, &params.project)
        .await
        .map_err(surreal_to_rpc)?
    else {
        return Ok(Vec::new());
    };
    let Some(project_id) = project.id else {
        return Ok(Vec::new());
    };
    let entries = kavach_surreal::list_by_project(&state.db, TABLE_ROADMAP, &project_id)
        .await
        .map_err(surreal_to_rpc)?;
    let dep_pool = kavach_surreal::list_all_by_table(&state.db, TABLE_ROADMAP)
        .await
        .map_err(surreal_to_rpc)?;
    let ready = entries
        .iter()
        .filter(|e| is_runnable_status(e.entry_status_str()) && deps_satisfied(e, &dep_pool))
        .map(|e| NextTaskResult {
            key: e.entry_key.clone(),
            title: e.title.clone(),
            status: e.entry_status_str().to_owned(),
        })
        .collect();
    Ok(ready)
}
