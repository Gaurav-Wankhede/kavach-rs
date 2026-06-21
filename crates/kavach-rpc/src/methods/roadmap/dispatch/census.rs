use super::super::readiness::{
    deps_satisfied, dep_index, is_in_cycle, is_runnable_status, is_umbrella,
};
use super::super::types::{NextOpenTaskParams, OpenSetCensus};
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;

const TABLE_ROADMAP: &str = "roadmap";

/// Census of the open set, splitting a BLOCKED remainder from an empty board.
///
/// `next_open_task`/`ready_set` both collapse "no runnable card" and "runnable
/// cards all waiting on unmet dependencies" to the same `None`/empty — so the
/// gate cannot decide between a clean `[ALL_BLOCKED]` stop and an
/// `[AUTO_CONTINUE]` PLAN nudge without this split.
///
/// `runnable` = cards in a dispatchable status (`todo`/`in_progress`).
/// `blocked`  = of those, the ones whose declared dependencies are not yet
///              done — pure topological waiting. There is no operator-gate bucket
///              (removed 2026-06-16): a card is either runnable, waiting on a
///              dependency, or deleted.
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
        // Umbrella cards are NEVER dispatch targets (epic container, counted via
        // children). Gates are no longer excluded — owner-gating abolished.
        if is_umbrella(&e.title) {
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
    // SNAPSHOT the roadmap-only counts BEFORE the TaskList fold below. These are
    // the DISPATCH-REACHABLE subset (this project's roadmap rows the dispatch probe
    // can actually serve in this lane). The refuse-stop keys off THESE, not the
    // post-fold totals — folding the GLOBAL TaskList into the stop decision would
    // trap any project session forever whenever the global list holds an open item.
    census.roadmap_runnable = census.runnable;
    census.roadmap_blocked = census.blocked;
    census.roadmap_cyclic = census.cyclic;

    // SECOND SOURCE: fold in the on-disk Claude Code TaskList store so the gate
    // sees BOTH backlogs. The roadmap table alone reported `runnable: 0` while
    // ~30 open TaskList items sat unseen, falsely "draining" the queue. A missing
    // store contributes (0, 0); an unresolved root is logged so a silent zero
    // stays observable rather than masquerading as a truly empty board.
    match super::tasklist::tasklist_root() {
        Some(root) => {
            let (tl_runnable, tl_blocked) = super::tasklist::tasklist_census(&root);
            census.runnable = census.runnable.saturating_add(tl_runnable);
            census.blocked = census.blocked.saturating_add(tl_blocked);
        }
        None => {
            tracing::warn!(
                "tasklist census skipped: no store root (HOME unset and {} unset) — \
                 census reflects roadmap table only",
                super::tasklist::TASKLIST_DIR_ENV_NAME
            );
        }
    }
    Ok(census)
}
