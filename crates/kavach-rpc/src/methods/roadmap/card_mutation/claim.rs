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
    // ATOMIC claim: a single conditional UPDATE (todo -> in_progress) is the
    // compare-and-set. The previous read-then-write was a TOCTOU race — two
    // sessions could both read "todo", both pass the check, and both write
    // in_progress, each believing it owned the card. With the CAS, SurrealDB
    // evaluates `entry_status = 'todo'` at write time, so exactly ONE racer
    // matches; the loser gets 0 rows updated and `claimed: false`.
    let updated = kavach_surreal::update_status_cas(
        &state.db,
        TABLE_ROADMAP,
        &project_id,
        &params.key,
        "todo",
        "in_progress",
    )
    .await
    .map_err(surreal_to_rpc)?;
    if updated > 0 {
        return Ok(ClaimCardResult {
            key: params.key,
            status: "in_progress".to_owned(),
            claimed: true,
        });
    }
    // CAS missed: either the key is absent or another session already moved it
    // off `todo`. Report the actual current status so the caller can distinguish
    // "already claimed" from "gone".
    let current = kavach_surreal::get_by_key(&state.db, TABLE_ROADMAP, &project_id, &params.key)
        .await
        .map_err(surreal_to_rpc)?;
    let current_status = current
        .as_ref()
        .map_or("", |e| e.entry_status_str())
        .to_owned();
    Ok(ClaimCardResult {
        key: params.key,
        status: current_status,
        claimed: false,
    })
}
