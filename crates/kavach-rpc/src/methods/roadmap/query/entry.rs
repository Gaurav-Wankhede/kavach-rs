use super::super::types::{EntryStatusParams, EntryStatusResult};
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;

const TABLE_ROADMAP: &str = "roadmap";

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when the database query fails.
pub async fn entry_status(
    state: &AppState,
    params: EntryStatusParams,
) -> Result<Option<EntryStatusResult>, ErrorObjectOwned> {
    let Some(project) = kavach_surreal::project_get_by_slug(&state.db, &params.project)
        .await
        .map_err(surreal_to_rpc)?
    else {
        return Ok(None);
    };
    let Some(project_id) = project.id else {
        return Ok(None);
    };
    let entry = kavach_surreal::get_by_key(&state.db, TABLE_ROADMAP, &project_id, &params.key)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(entry.map(|e| EntryStatusResult {
        status: e.entry_status_str().to_owned(),
    }))
}
