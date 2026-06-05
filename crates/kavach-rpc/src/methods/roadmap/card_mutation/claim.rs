use super::super::types::{ClaimCardParams, ClaimCardResult};
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;

const TABLE_ROADMAP: &str = "roadmap";

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when the database query fails.
pub async fn claim_card(
    state: &AppState,
    params: ClaimCardParams,
) -> Result<ClaimCardResult, ErrorObjectOwned> {
    let Some(project) = kavach_surreal::project_get_by_slug(&state.db, &params.project)
        .await
        .map_err(surreal_to_rpc)?
    else {
        return Ok(ClaimCardResult {
            key: params.key,
            status: String::new(),
            claimed: false,
        });
    };
    let Some(project_id) = project.id else {
        return Ok(ClaimCardResult {
            key: params.key,
            status: String::new(),
            claimed: false,
        });
    };
    let current = kavach_surreal::get_by_key(&state.db, TABLE_ROADMAP, &project_id, &params.key)
        .await
        .map_err(surreal_to_rpc)?;
    let current_status = current
        .as_ref()
        .map_or("", |e| e.entry_status_str())
        .to_owned();
    if current_status != "todo" {
        return Ok(ClaimCardResult {
            key: params.key,
            status: current_status,
            claimed: false,
        });
    }
    let updated = kavach_surreal::update_status(
        &state.db,
        TABLE_ROADMAP,
        &project_id,
        &params.key,
        "in_progress",
    )
    .await
    .map_err(surreal_to_rpc)?;
    Ok(ClaimCardResult {
        key: params.key,
        status: "in_progress".to_owned(),
        claimed: updated > 0,
    })
}
