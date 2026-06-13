use super::super::readiness::{deps_satisfied, dep_index, is_in_cycle, is_runnable_status};
use super::super::types::{NextOpenTaskParams, OpenSetCensus};
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;

const TABLE_ROADMAP: &str = "roadmap";

/// Census of the open set, splitting a BLOCKED remainder from an empty board.
///
/// `next_open_task`/`ready_set` both collapse "no runnable card" and "runnable
/// cards all blocked by unmet deps / owner-gating" to the same `None`/empty — so
/// the gate cannot decide between a clean `[ALL_BLOCKED]` stop and an
/// `[AUTO_CONTINUE]` PLAN nudge without this split.
///
/// `runnable` = cards in a dispatchable status (`todo`/`in_progress`).
/// `blocked`  = of those, the ones held back by unmet deps or an agent-gate
///              (`AGENT_BLOCKED`/owner-only) — i.e. real work the AI cannot start.
///
/// # Errors
/// Returns an RPC `ErrorObjectOwned` when the database query fails.
pub async fn open_set_census(
    state: &AppState,
    params: NextOpenTaskParams,
) -> Result<OpenSetCensus, ErrorObjectOwned> {
    let Some(project) = kavach_surreal::project_get_by_slug(&state.db, &params.project)
        .await
        .map_err(surreal_to_rpc)?
    else {
        return Ok(OpenSetCensus::default());
    };
    let Some(project_id) = project.id else {
        return Ok(OpenSetCensus::default());
    };
    let entries = kavach_surreal::list_by_project(&state.db, TABLE_ROADMAP, &project_id)
        .await
        .map_err(surreal_to_rpc)?;
    let dep_pool = kavach_surreal::list_all_by_table(&state.db, TABLE_ROADMAP)
        .await
        .map_err(surreal_to_rpc)?;
    // Index the GLOBAL dep pool once: cycle detection follows declared-dep edges
    // across every project (dep keys are a global key space, per `deps_satisfied`).
    let by_key = dep_index(&dep_pool);
    let mut census = OpenSetCensus::default();
    for e in &entries {
        if !is_runnable_status(e.entry_status_str()) {
            continue;
        }
        census.runnable = census.runnable.saturating_add(1);
        // A cyclic card can never satisfy deps; count it as cyclic (NOT blocked)
        // so it cannot forge a legitimate-looking `[ALL_BLOCKED]` clean-stop.
        if is_in_cycle(&e.entry_key, &by_key) {
            census.cyclic = census.cyclic.saturating_add(1);
            census.cyclic_keys.push(e.entry_key.clone());
        } else if !deps_satisfied(e, &dep_pool) {
            census.blocked = census.blocked.saturating_add(1);
        }
    }
    Ok(census)
}
