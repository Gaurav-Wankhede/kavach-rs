use super::super::types::{ClaimCardParams, ClaimCardResult};
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::lease::AcquireOutcome;

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
        return Ok(not_claimed(params.key, String::new()));
    };
    let Some(project_id) = project.id else {
        return Ok(not_claimed(params.key, String::new()));
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
        return claim_won(state, &project_id, params).await;
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
    Ok(not_claimed(params.key, current_status))
}

/// The status-CAS won (`todo -> in_progress`). Fuse the occupancy lease so the
/// winning session OWNS the card with a renewable TTL — without this the card
/// has no holder/heartbeat and a hung holder cannot be told from a crashed one, so
/// a second live session would resume it (the concurrent double-resume defect).
///
/// A legacy caller with no `session_id` keeps the pre-lease status-only claim.
/// If the lease acquire FAILS after the status flip, the card must NOT be left
/// half-claimed (`in_progress`, holder-less) — roll the status back to `todo` so it
/// returns to the dispatch pool, and report `claimed: false`. Fail closed: a
/// claim is "won" only when BOTH the status flip and the lease succeed.
async fn claim_won(
    state: &AppState,
    project_id: &surrealdb_types::RecordId,
    params: ClaimCardParams,
) -> Result<ClaimCardResult, ErrorObjectOwned> {
    let Some(session_id) = params.session_id.as_deref().filter(|s| !s.is_empty()) else {
        // Legacy status-only claim: no holder, no lease (pre-lease behaviour).
        return Ok(claimed(params.key, None));
    };
    match kavach_surreal::lease::acquire(&state.db, TABLE_ROADMAP, &params.key, session_id)
        .await
        .map_err(surreal_to_rpc)?
    {
        AcquireOutcome::Acquired(lease) => Ok(claimed(params.key, Some(lease.epoch))),
        // We won the status flip but a live lease is held by ANOTHER session — an
        // inconsistent interleave (e.g. a prior holder mid-reclaim). Roll the
        // status back so no card is stranded holder-less in_progress, and lose.
        AcquireOutcome::HeldBy { .. } => {
            roll_back_to_todo(state, project_id, &params.key).await?;
            Ok(not_claimed(params.key, "todo".to_owned()))
        }
    }
}

/// Undo a status flip whose lease did not land. Best-effort CAS back
/// `in_progress -> todo`; a failure is logged, not swallowed, since a stuck
/// holder-less card is a dispatch leak the time-based reclaim still recovers.
async fn roll_back_to_todo(
    state: &AppState,
    project_id: &surrealdb_types::RecordId,
    key: &str,
) -> Result<(), ErrorObjectOwned> {
    if let Err(e) = kavach_surreal::update_status_cas(
        &state.db,
        TABLE_ROADMAP,
        project_id,
        key,
        "in_progress",
        "todo",
    )
    .await
    {
        tracing::warn!(error = %e, key, "claim rollback to todo failed; reclaim sweep will recover the holder-less card");
    }
    Ok(())
}

/// A won claim: card is `in_progress`, owned by this session. `epoch` carries the
/// lease fence (`Some`) or is `None` for a legacy status-only claim.
fn claimed(key: String, epoch: Option<i64>) -> ClaimCardResult {
    ClaimCardResult {
        key,
        status: "in_progress".to_owned(),
        claimed: true,
        epoch,
    }
}

/// A lost/absent claim: report the card's actual current status; no lease taken.
const fn not_claimed(key: String, status: String) -> ClaimCardResult {
    ClaimCardResult {
        key,
        status,
        claimed: false,
        epoch: None,
    }
}
