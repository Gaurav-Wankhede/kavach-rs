// split: intentional - cohesive graph-entity RPC group (entity upsert/find + relate/get_related)
// JSON-RPC method handlers for dynamic graph entity + relationship ops.
// Used by kavach-engine rag_router (and forthcoming session_start) to record
// session→skill, skill→skill cross_invoke, file→skill INVOKE edges
// without spawning tokio per call.
use crate::error::{invalid_params, surreal_to_rpc};
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::{
    Entity, RelatedRow, graph_find_entity, graph_get_related, graph_relate_dynamic,
    graph_upsert_entity,
};
use serde::{Deserialize, Serialize};
use surrealdb_types::RecordId;

/// Parameters for upserting a graph entity.
#[non_exhaustive]
#[derive(Debug, Deserialize)]
pub struct EntityUpsertParams {
    pub entity_type: String,
    pub name: String,
}

/// Result containing the ID of an entity.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize)]
pub struct EntityIdResult {
    pub id: String,
}

/// Parameters for finding a graph entity.
#[non_exhaustive]
#[derive(Debug, Deserialize)]
pub struct EntityFindParams {
    pub entity_type: String,
    pub name: String,
}

/// Parameters for relating two graph entities.
#[non_exhaustive]
#[derive(Debug, Deserialize)]
pub struct RelateParams {
    pub from: String,
    pub to: String,
    pub rel_type: String,
    pub weight: Option<f64>,
}

/// Parameters for getting related entities.
#[non_exhaustive]
#[derive(Debug, Deserialize)]
pub struct GetRelatedParams {
    pub from: String,
    pub limit: Option<usize>,
}

fn parse_record_id(s: &str) -> Result<RecordId, ErrorObjectOwned> {
    match s.split_once(':') {
        Some((table, key)) => Ok(RecordId {
            table: table.to_owned().into(),
            key: surrealdb_types::RecordIdKey::String(key.to_owned()),
        }),
        None => Err(invalid_params(format!(
            "expected RecordId 'table:id', got: {s}"
        ))),
    }
}

/// Upsert a graph entity, creating it if not present.
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub async fn entity_upsert(
    state: &AppState,
    params: EntityUpsertParams,
) -> Result<EntityIdResult, ErrorObjectOwned> {
    let id = graph_upsert_entity(&state.db, &params.entity_type, &params.name)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(EntityIdResult {
        id: format!("{id:?}"),
    })
}

/// Find a graph entity by type and name.
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub async fn entity_find(
    state: &AppState,
    params: EntityFindParams,
) -> Result<Option<Entity>, ErrorObjectOwned> {
    graph_find_entity(&state.db, &params.entity_type, &params.name)
        .await
        .map_err(surreal_to_rpc)
}

/// Add a relationship between two graph entities.
///
/// # Errors
///
/// Returns an error if parsing record IDs fails or the database operation fails.
pub async fn add_relationship(
    state: &AppState,
    params: RelateParams,
) -> Result<&'static str, ErrorObjectOwned> {
    let from = parse_record_id(&params.from)?;
    let to = parse_record_id(&params.to)?;
    let weight = params.weight.unwrap_or(1.0);
    graph_relate_dynamic(&state.db, &from, &to, &params.rel_type, weight)
        .await
        .map_err(surreal_to_rpc)?;
    Ok("ok")
}

/// Get entities related to a given entity.
///
/// # Errors
///
/// Returns an error if parsing the record ID fails or the database operation fails.
pub async fn get_related(
    state: &AppState,
    params: GetRelatedParams,
) -> Result<Vec<RelatedRow>, ErrorObjectOwned> {
    let from = parse_record_id(&params.from)?;
    let limit = params.limit.unwrap_or(50);
    graph_get_related(&state.db, &from, limit)
        .await
        .map_err(surreal_to_rpc)
}
