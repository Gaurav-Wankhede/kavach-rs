use super::super::types::{ListTitlesParams, NextOpenTaskParams, NextTaskResult, TitleRow};
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;

const TABLE_ROADMAP: &str = "roadmap";

// ALGO: ListThenTruncate
// PROBLEM_CLASS: stream
// TIME: O(n) read + O(limit) truncate | SPACE: O(n)
// YEAR: 2026 | SEARCHED: 2026-05
// TRADEOFF: read-all-then-truncate when limit small relative to n
// BENCHMARK: list_by_project caps at 5000 rows; sub-10ms typical
/// # Errors
/// Returns an RPC `ErrorObjectOwned` when the database query fails.
pub async fn list_titles(
    state: &AppState,
    params: ListTitlesParams,
) -> Result<Vec<TitleRow>, ErrorObjectOwned> {
    const DEFAULT_LIMIT: usize = 10;
    let limit = match params.limit {
        Some(l) if l > 0 => l,
        Some(_) | None => DEFAULT_LIMIT,
    };
    let category = match params.category.as_deref() {
        Some(c) if !c.is_empty() => c,
        Some(_) | None => TABLE_ROADMAP,
    };
    let Some(project) = kavach_surreal::project_get_by_slug(&state.db, &params.project)
        .await
        .map_err(surreal_to_rpc)?
    else {
        return Ok(Vec::new());
    };
    let Some(project_id) = project.id else {
        return Ok(Vec::new());
    };
    let entries = kavach_surreal::list_by_project(&state.db, category, &project_id)
        .await
        .map_err(surreal_to_rpc)?;
    let mut rows: Vec<TitleRow> = entries
        .into_iter()
        .map(|e| TitleRow {
            category: e.category_str().to_owned(),
            key: e.entry_key.clone(),
            title: e.title.clone(),
            entry_status: e.entry_status_str().to_owned(),
        })
        .collect();
    rows.truncate(limit);
    Ok(rows)
}

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when the database query fails.
pub async fn list_done_cards(
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
    let entries = kavach_surreal::list_by_project(&state.db, TABLE_ROADMAP, &project_id)
        .await
        .map_err(surreal_to_rpc)?;
    let done = entries
        .iter()
        .filter(|e| e.entry_status_str() == "done")
        .map(|e| NextTaskResult {
            key: e.entry_key.clone(),
            title: e.title.clone(),
            status: e.entry_status_str().to_owned(),
        })
        .collect();
    Ok(done)
}
