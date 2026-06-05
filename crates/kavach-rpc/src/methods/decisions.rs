// split: intentional - cohesive arch/algo decision RPC group (list + upsert)
// JSON-RPC method handlers for arch/algo decision lookups + recording.
// Used by kavach-engine pre-write guards (read) and post-tool recorders (write).
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::{
    AlgoDecision, AlgoUpsertParams, ArchDecision, ArchUpsertParams, algo_list_recent, algo_upsert,
    arch_list_recent, arch_upsert,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC wire DTO: constructed by handler"
)]
pub struct ListParams {
    pub project: String,
    pub limit: Option<usize>,
}

/// List recent algorithm decisions for a project.
///
/// # Errors
/// Returns `surreal_to_rpc` error if the project lookup fails.
pub async fn algo_list(
    state: &AppState,
    params: ListParams,
) -> Result<Vec<AlgoDecision>, ErrorObjectOwned> {
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
    algo_list_recent(&state.db, &project_id, limit)
        .await
        .map_err(surreal_to_rpc)
}

#[derive(Debug, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC wire DTO: constructed by handler"
)]
pub struct ArchUpsertRpcParams {
    pub project: String,
    pub pattern: String,
    pub scope: String,
    pub cap_choice: Option<String>,
    pub failure_mode: String,
    pub tradeoff: String,
    pub file_path: String,
    pub search_year: i32,
    pub search_month: i32,
}

#[derive(Debug, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC wire DTO: constructed by handler"
)]
pub struct AlgoUpsertRpcParams {
    pub project: String,
    pub problem_class: String,
    pub chosen: String,
    pub time_complexity: String,
    pub space_complexity: String,
    pub file_path: String,
    pub search_year: i32,
    pub search_month: i32,
}

#[derive(Debug, Clone, Serialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC response DTO: constructed by handler"
)]
pub struct UpsertResult {
    pub id: String,
}

/// Upsert an architecture decision.
///
/// # Errors
/// Returns `surreal_to_rpc` error if the project lookup or upsert fails.
pub async fn arch_upsert_rpc(
    state: &AppState,
    params: ArchUpsertRpcParams,
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
    let upsert_params = ArchUpsertParams {
        project: project_id,
        pattern: &params.pattern,
        scope: &params.scope,
        cap_choice: params.cap_choice.as_deref(),
        failure_mode: &params.failure_mode,
        tradeoff: &params.tradeoff,
        file_path: &params.file_path,
        search_year: params.search_year,
        search_month: params.search_month,
    };
    let id = arch_upsert(&state.db, &upsert_params)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(Some(UpsertResult {
        id: format!("{id:?}"),
    }))
}

/// Upsert an algorithm decision.
///
/// # Errors
/// Returns `surreal_to_rpc` error if the project lookup or upsert fails.
pub async fn algo_upsert_rpc(
    state: &AppState,
    params: AlgoUpsertRpcParams,
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
    let upsert_params = AlgoUpsertParams {
        project: project_id,
        problem_class: &params.problem_class,
        chosen: &params.chosen,
        time_complexity: &params.time_complexity,
        space_complexity: &params.space_complexity,
        file_path: &params.file_path,
        search_year: params.search_year,
        search_month: params.search_month,
    };
    let id = algo_upsert(&state.db, &upsert_params)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(Some(UpsertResult {
        id: format!("{id:?}"),
    }))
}

/// List recent architecture decisions for a project.
///
/// # Errors
/// Returns `surreal_to_rpc` error if the project lookup fails.
pub async fn arch_list(
    state: &AppState,
    params: ListParams,
) -> Result<Vec<ArchDecision>, ErrorObjectOwned> {
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
    arch_list_recent(&state.db, &project_id, limit)
        .await
        .map_err(surreal_to_rpc)
}
