use super::super::types::{ClaimCardParams, VerifyCardResult};
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;

const TABLE_ROADMAP: &str = "roadmap";

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when the database query fails.
pub async fn verify_card(
    state: &AppState,
    params: ClaimCardParams,
) -> Result<VerifyCardResult, ErrorObjectOwned> {
    let Some(project) = kavach_surreal::project_get_by_slug(&state.db, &params.project)
        .await
        .map_err(surreal_to_rpc)?
    else {
        return Ok(VerifyCardResult {
            key: params.key,
            status: String::new(),
            verified: false,
        });
    };
    let Some(project_id) = project.id else {
        return Ok(VerifyCardResult {
            key: params.key,
            status: String::new(),
            verified: false,
        });
    };
    let current = kavach_surreal::get_by_key(&state.db, TABLE_ROADMAP, &project_id, &params.key)
        .await
        .map_err(surreal_to_rpc)?;
    let current_status = current
        .as_ref()
        .map_or("", |e| e.entry_status_str())
        .to_owned();
    if current_status != "done" {
        return Ok(VerifyCardResult {
            key: params.key,
            status: current_status,
            verified: false,
        });
    }
    let updated = kavach_surreal::update_status(
        &state.db,
        TABLE_ROADMAP,
        &project_id,
        &params.key,
        "verified",
    )
    .await
    .map_err(surreal_to_rpc)?;
    Ok(VerifyCardResult {
        key: params.key,
        status: "verified".to_owned(),
        verified: updated > 0,
    })
}
