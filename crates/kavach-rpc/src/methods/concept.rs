// split: intentional — RPC method namespace for L0 concept verbs (add/link/find/search/list) + evidence gate; mirrors methods/graph.rs pattern (5 verbs, 1 file).
// jsonrpsee 0.24 — see https://docs.rs/jsonrpsee/0.24
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::{
    Entity, graph_delete_concept, graph_delete_concepts_by_prefix, graph_find_concept,
    graph_list_concepts, graph_relate_concepts, graph_search_concepts_fts, graph_upsert_concept,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct AddParams {
    pub name: String,
    pub display: String,
    pub desc: String,
    pub tags: Option<Vec<String>>,
    pub sources: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC result DTO constructed at handler boundary"
)]
pub struct IdResult {
    pub id: String,
}

const AUTO_HARVEST_TAG: &str = "auto-harvested";

fn require_evidence(
    desc: &str,
    sources: &[String],
    tags: &[String],
) -> Result<(), ErrorObjectOwned> {
    if tags.iter().any(|t| t == AUTO_HARVEST_TAG) {
        return Ok(());
    }
    if !sources.is_empty() {
        return Ok(());
    }
    if desc.contains("http://") || desc.contains("https://") {
        return Ok(());
    }
    Err(ErrorObjectOwned::owned(
        -32001,
        "evidence_required: concept.add needs a source URL. \
         Pass --sources URL or include http(s):// in --desc. \
         Auto-harvest bypass: tags=['auto-harvested']. (§EVIDENCE law)",
        None::<()>,
    ))
}

/// Add a new concept to the knowledge graph.
///
/// # Errors
///
/// Returns `ErrorObjectOwned` if the concept evidence gate fails (no source URL or auto-harvest bypass) or if the database operation fails.
pub async fn add(state: &AppState, p: AddParams) -> Result<IdResult, ErrorObjectOwned> {
    let tags = p.tags.unwrap_or_default();
    let sources = p.sources.unwrap_or_default();
    require_evidence(&p.desc, &sources, &tags)?;
    let id = graph_upsert_concept(&state.db, &p.name, &p.display, &p.desc, &tags, &sources)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(IdResult {
        id: format!("{id:?}"),
    })
}

#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct LinkParams {
    pub from: String,
    pub edge: String,
    pub to: String,
}

/// Create a relationship edge between two concepts.
///
/// # Errors
///
/// Returns `ErrorObjectOwned` if the database operation fails.
pub async fn link(state: &AppState, p: LinkParams) -> Result<&'static str, ErrorObjectOwned> {
    graph_relate_concepts(&state.db, &p.from, &p.edge, &p.to)
        .await
        .map_err(surreal_to_rpc)?;
    Ok("ok")
}

#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct FindParams {
    pub name: String,
}

/// Find a concept by name.
///
/// # Errors
///
/// Returns `ErrorObjectOwned` if the database operation fails.
pub async fn find(state: &AppState, p: FindParams) -> Result<Option<Entity>, ErrorObjectOwned> {
    graph_find_concept(&state.db, &p.name)
        .await
        .map_err(surreal_to_rpc)
}

#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct SearchParams {
    pub query: String,
    pub limit: Option<usize>,
}

/// Search concepts by full-text query.
///
/// # Errors
///
/// Returns `ErrorObjectOwned` if the database operation fails.
pub async fn search(state: &AppState, p: SearchParams) -> Result<Vec<Entity>, ErrorObjectOwned> {
    let limit = p.limit.unwrap_or(20);
    graph_search_concepts_fts(&state.db, &p.query, limit)
        .await
        .map_err(surreal_to_rpc)
}

#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct ListParams {
    pub limit: Option<usize>,
}

/// List all concepts with optional limit.
///
/// # Errors
///
/// Returns `ErrorObjectOwned` if the database operation fails.
pub async fn list(state: &AppState, p: ListParams) -> Result<Vec<Entity>, ErrorObjectOwned> {
    let limit = p.limit.unwrap_or(50);
    graph_list_concepts(&state.db, limit)
        .await
        .map_err(surreal_to_rpc)
}

#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct DeleteParams {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC result DTO constructed at handler boundary"
)]
pub struct DeleteResult {
    pub removed: i64,
}

/// Delete a concept by name.
///
/// # Errors
///
/// Returns `ErrorObjectOwned` if the database operation fails.
pub async fn delete(state: &AppState, p: DeleteParams) -> Result<DeleteResult, ErrorObjectOwned> {
    let n = graph_delete_concept(&state.db, &p.name)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(DeleteResult { removed: n })
}

#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct DeleteByPrefixParams {
    pub prefix: String,
    pub confirm: bool,
}

/// Delete concepts by prefix with confirmation gate.
///
/// # Errors
///
/// Returns `ErrorObjectOwned` if the database operation fails or if confirm=false.
pub async fn delete_by_prefix(
    state: &AppState,
    p: DeleteByPrefixParams,
) -> Result<DeleteResult, ErrorObjectOwned> {
    if !p.confirm {
        return Err(ErrorObjectOwned::owned(
            -32002,
            "confirm_required: bulk delete needs confirm=true to prevent accidental wipes",
            None::<()>,
        ));
    }
    let n = graph_delete_concepts_by_prefix(&state.db, &p.prefix)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(DeleteResult { removed: n })
}
