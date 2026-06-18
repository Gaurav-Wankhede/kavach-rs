use super::super::readiness::{deps_satisfied, is_gate, is_runnable_status, is_umbrella};
use super::super::types::{NextOpenTaskParams, NextTaskResult};
use super::lane_pick::{lane_matches, pick_in_lane};
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
    // Reclaim crash-orphaned cards BEFORE selecting — closes harness-loop L1: a
    // session that claimed a card then crashed left it `in_progress` with a
    // lapsed lease, un-dispatchable forever. Reclaim gates dispatch so the orphan
    // is reset to `todo` and reconsidered this very call. Best-effort: a reclaim
    // failure must NOT block dispatch of the cards that are already runnable, but
    // it is logged (not silently swallowed) so a persistent reclaim fault is
    // diagnosable rather than an invisible backlog leak.
    if let Err(e) = kavach_surreal::lease::reclaim_orphaned_in_progress(&state.db).await {
        tracing::warn!(error = %e, "crash-orphan reclaim failed before dispatch; proceeding with current runnable set");
    }
    let mut entries = kavach_surreal::list_by_project(&state.db, TABLE_ROADMAP, &project_id)
        .await
        .map_err(surreal_to_rpc)?;
    // E3 priority-ceiling: re-rank so a low-urgency BLOCKER inherits the priority
    // of its most-urgent dependent — otherwise a pri-1 card starves forever
    // behind its pri-9 blocker (priority inversion). The DB read sorts by RAW
    // priority; this lifts blockers to their ceiling before selection. SOURCE:
    // decision.harness.loop-edge-cases-and-db-optimization E3.
    super::priority_ceiling::sort_by_effective_priority(&mut entries);
    let dep_pool = kavach_surreal::list_all_by_table(&state.db, TABLE_ROADMAP)
        .await
        .map_err(surreal_to_rpc)?;
    // Two-pass lane-affinity dispatch. `entries` is priority-ordered, so the
    // first match per pass is the best card. Pass 1: the session's OWN lane.
    // Pass 2: the unlaned (NULL) general backlog. A foreign lane is NEVER
    // inspected. With no session lane, pass 1 matches everything and pass 2 is a
    // no-op — byte-identical to the pre-lane single loop.
    let want = params.lane.as_deref();
    let me = params.session_id.as_deref().unwrap_or_default();
    let selected = pick_in_lane(&entries, &dep_pool, me, |e| lane_matches(e, want))
        .or_else(|| pick_in_lane(&entries, &dep_pool, me, |e| e.lane.is_none()));
    if selected.is_some() {
        return Ok(selected);
    }
    // E2: picking NOTHING can mean an unsatisfiable DEPENDS_ON CYCLE (A→B→A) —
    // every card in it waits on the next, so none is ever runnable and the loop
    // would stall silently. Surface the cycle as a [DAG_CYCLE] allow-stop NAMING
    // the keys, instead of an invisible "picked neither". SOURCE:
    // decision.harness.loop-edge-cases-and-db-optimization E2.
    if let Some(cycle) = super::dag_cycle::detect_cycle(&entries) {
        return Ok(Some(NextTaskResult {
            key: "[DAG_CYCLE]".to_owned(),
            title: super::dag_cycle::cycle_message(&cycle),
            status: "dag_cycle".to_owned(),
        }));
    }
    Ok(None)
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
    // Reclaim crash-orphans before censusing the ready set (see `next_open_task`).
    if let Err(e) = kavach_surreal::lease::reclaim_orphaned_in_progress(&state.db).await {
        tracing::warn!(error = %e, "crash-orphan reclaim failed before ready_set; proceeding with current runnable set");
    }
    let entries = kavach_surreal::list_by_project(&state.db, TABLE_ROADMAP, &project_id)
        .await
        .map_err(surreal_to_rpc)?;
    let dep_pool = kavach_surreal::list_all_by_table(&state.db, TABLE_ROADMAP)
        .await
        .map_err(surreal_to_rpc)?;
    let ready = entries
        .iter()
        .filter(|e| {
            is_runnable_status(e.entry_status_str())
                && !is_umbrella(&e.title)
                && !is_gate(&e.title)
                && deps_satisfied(e, &dep_pool)
        })
        .map(|e| NextTaskResult {
            key: e.entry_key.clone(),
            title: e.title.clone(),
            status: e.entry_status_str().to_owned(),
        })
        .collect();
    Ok(ready)
}
