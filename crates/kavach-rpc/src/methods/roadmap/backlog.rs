use super::readiness::{
    deps_satisfied, is_gate, is_needs_decomposition, is_runnable_status, is_umbrella,
};
use super::types::{NextOpenTaskParams, NextTaskResult};
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;

const TABLE_ROADMAP: &str = "roadmap";
const HUNT_KEY_PREFIX: &str = "hunt.";

fn backlog_priority_class(key: &str) -> u8 {
    if key.starts_with(HUNT_KEY_PREFIX) {
        0
    } else if key.starts_with("P0") {
        1
    } else if key.starts_with("P1") {
        2
    } else {
        3
    }
}

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when the database query fails.
pub async fn promote_next_backlog(
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
    // Backlog tier applies the SAME gate/umbrella/decomp exclusions as the primary selectors.
    let mut ready: Vec<_> = entries
        .into_iter()
        .filter(|e| {
            is_runnable_status(e.entry_status_str())
                && deps_satisfied(e, &dep_pool)
                && !is_gate(&e.title)
                && !is_umbrella(&e.title)
                && !is_needs_decomposition(&e.title)
        })
        .collect();
    if ready.is_empty() {
        return Ok(None);
    }
    ready.sort_by(|a, b| {
        backlog_priority_class(&a.entry_key)
            .cmp(&backlog_priority_class(&b.entry_key))
            .then_with(|| match (a.created_at, b.created_at) {
                (Some(x), Some(y)) => x.cmp(&y),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            })
    });
    let Some(picked) = ready.into_iter().next() else {
        return Ok(None);
    };
    Ok(Some(NextTaskResult {
        key: picked.entry_key.clone(),
        title: picked.title.clone(),
        status: picked.entry_status_str().to_owned(),
    }))
}
