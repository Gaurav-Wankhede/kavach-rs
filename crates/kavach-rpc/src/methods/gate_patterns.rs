// split: intentional - cohesive gate_pattern RPC group (find_autonomous + upsert + list_hot)
// JSON-RPC method handlers for gate_pattern store.
// Used by kavach-engine post_tool_failure gate (find/upsert) and session_start (list_hot).
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::{
    GatePattern, GatePatternUpsertParams, gate_pattern_find_autonomous, gate_pattern_list_hot,
    gate_pattern_upsert,
};
use serde::{Deserialize, Serialize};

/// Wire DTO for finding gate patterns by error.
#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct FindParams {
    pub project: String,
    pub error: String,
    pub tool_name: String,
}

/// Wire DTO for upserting gate patterns.
#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct UpsertRpcParams {
    pub project: String,
    pub error_tokens: String,
    pub fix_strategy: String,
    pub imperative_rewrite: String,
    pub dsa_rationale: String,
    pub tool_name: String,
    pub gate_name: String,
}

/// Wire DTO for upsert results.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct UpsertResult {
    pub id: String,
}

/// Wire DTO for listing hot gate patterns.
#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct ListHotParams {
    pub project: String,
    pub limit: Option<usize>,
}

/// List the hottest gate patterns for a project.
///
/// # Errors
///
/// Returns an error if the database query fails or the project slug lookup fails.
pub async fn list_hot(
    state: &AppState,
    params: ListHotParams,
) -> Result<Vec<GatePattern>, ErrorObjectOwned> {
    let limit = params.limit.unwrap_or(5);
    let Some(project) = kavach_surreal::project_get_by_slug(&state.db, &params.project)
        .await
        .map_err(surreal_to_rpc)?
    else {
        return Ok(Vec::new());
    };
    let Some(project_id) = project.id else {
        return Ok(Vec::new());
    };
    gate_pattern_list_hot(&state.db, &project_id, limit)
        .await
        .map_err(surreal_to_rpc)
}

/// Find an autonomous gate pattern matching the given error signature.
///
/// # Errors
///
/// Returns an error if the database query fails or the project slug lookup fails.
pub async fn find_autonomous(
    state: &AppState,
    params: FindParams,
) -> Result<Option<GatePattern>, ErrorObjectOwned> {
    let Some(project) = kavach_surreal::project_get_by_slug(&state.db, &params.project)
        .await
        .map_err(surreal_to_rpc)?
    else {
        return Ok(None);
    };
    let Some(project_id) = project.id else {
        return Ok(None);
    };
    gate_pattern_find_autonomous(&state.db, &project_id, &params.error, &params.tool_name)
        .await
        .map_err(surreal_to_rpc)
}

/// Upsert a gate pattern with the given error tokens and fix strategy.
///
/// # Errors
///
/// Returns an error if the database query fails or the project slug lookup fails.
pub async fn upsert(
    state: &AppState,
    params: UpsertRpcParams,
) -> Result<Option<UpsertResult>, ErrorObjectOwned> {
    let Some(project) = kavach_surreal::project_get_by_slug(&state.db, &params.project)
        .await
        .map_err(surreal_to_rpc)?
    else {
        return Ok(None);
    };
    let Some(project_id) = project.id else {
        return Ok(None);
    };
    let upsert_params = GatePatternUpsertParams {
        project: project_id,
        error_tokens: &params.error_tokens,
        fix_strategy: &params.fix_strategy,
        imperative_rewrite: &params.imperative_rewrite,
        dsa_rationale: &params.dsa_rationale,
        tool_name: &params.tool_name,
        gate_name: &params.gate_name,
    };
    let id = gate_pattern_upsert(&state.db, &upsert_params)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(Some(UpsertResult {
        id: format!("{id:?}"),
    }))
}
