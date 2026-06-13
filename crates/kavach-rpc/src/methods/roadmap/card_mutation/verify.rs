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
    // ATOMIC verify: single conditional UPDATE (done -> verified) is the CAS,
    // same fix as claim_card. The prior read-then-write let a second session
    // verify a card the first was mid-transition on. Only one racer matches the
    // `entry_status = 'done'` predicate at write time.
    let updated = kavach_surreal::update_status_cas(
        &state.db,
        TABLE_ROADMAP,
        &project_id,
        &params.key,
        "done",
        "verified",
    )
    .await
    .map_err(surreal_to_rpc)?;
    if updated > 0 {
        return Ok(VerifyCardResult {
            key: params.key,
            status: "verified".to_owned(),
            verified: true,
        });
    }
    // CAS missed: report the real current status (absent, or not yet `done`).
    let current = kavach_surreal::get_by_key(&state.db, TABLE_ROADMAP, &project_id, &params.key)
        .await
        .map_err(surreal_to_rpc)?;
    let current_status = current
        .as_ref()
        .map_or("", |e| e.entry_status_str())
        .to_owned();
    Ok(VerifyCardResult {
        key: params.key,
        status: current_status,
        verified: false,
    })
}
