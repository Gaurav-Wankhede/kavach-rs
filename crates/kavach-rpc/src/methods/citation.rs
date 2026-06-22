// split: intentional — RPC namespace for citation verbs (add/get/list/link/traverse/refresh); mirrors methods/concept.rs.
// jsonrpsee 0.24 — see https://docs.rs/jsonrpsee/0.24
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::{
    Citation, CitationMeta, UpsertCitation, citation_citations_for_nodes, citation_get,
    citation_list, citation_merge_node, citation_reward, citation_traverse, citation_upsert,
};
use serde::{Deserialize, Serialize};
use surrealdb_types::RecordId;

#[derive(Debug, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at handler boundary")]
pub struct AddParams {
    pub project: String,
    pub entry_key: String,
    pub name: String,
    pub metadata: Vec<CitationMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC result DTO at handler boundary")]
pub struct IdResult {
    pub id: String,
}

/// Add or refresh a citation keyed by (project, `entry_key`).
///
/// # Errors
/// Returns `ErrorObjectOwned` when a metadata URL is empty or the DB fails.
pub async fn add(state: &AppState, p: AddParams) -> Result<IdResult, ErrorObjectOwned> {
    let project = RecordId::new("project", p.project);
    let id = citation_upsert(
        &state.db,
        &UpsertCitation {
            project,
            entry_key: &p.entry_key,
            name: &p.name,
            metadata: p.metadata,
        },
    )
    .await
    .map_err(surreal_to_rpc)?;
    Ok(IdResult { id: format!("{id:?}") })
}

#[derive(Debug, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at handler boundary")]
pub struct GetParams {
    pub project: String,
    pub entry_key: String,
}

/// Fetch one citation (bumps `access_count`).
///
/// # Errors
/// Returns `ErrorObjectOwned` when the DB fails.
pub async fn get(state: &AppState, p: GetParams) -> Result<Option<Citation>, ErrorObjectOwned> {
    let project = RecordId::new("project", p.project);
    citation_get(&state.db, &project, &p.entry_key)
        .await
        .map_err(surreal_to_rpc)
}

#[derive(Debug, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at handler boundary")]
pub struct ListParams {
    pub project: String,
}

/// List every citation for a project, newest-updated first.
///
/// # Errors
/// Returns `ErrorObjectOwned` when the DB fails.
pub async fn list(state: &AppState, p: ListParams) -> Result<Vec<Citation>, ErrorObjectOwned> {
    let project = RecordId::new("project", p.project);
    citation_list(&state.db, &project)
        .await
        .map_err(surreal_to_rpc)
}

#[derive(Debug, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at handler boundary")]
pub struct LinkParams {
    pub node: String,
    pub citation: String,
}

fn parse_record(s: &str) -> Result<RecordId, ErrorObjectOwned> {
    s.split_once(':')
        .map(|(table, key)| RecordId::new(table, key))
        .ok_or_else(|| {
            ErrorObjectOwned::owned(-32602, "expected 'table:key' record id", None::<()>)
        })
}

/// Merge a learning/decision/roadmap node into a citation (`->cite->`, idempotent).
///
/// # Errors
/// Returns `ErrorObjectOwned` on a malformed record id or DB failure.
pub async fn link(state: &AppState, p: LinkParams) -> Result<&'static str, ErrorObjectOwned> {
    let node = parse_record(&p.node)?;
    let citation = parse_record(&p.citation)?;
    citation_merge_node(&state.db, &node, &citation)
        .await
        .map_err(surreal_to_rpc)?;
    Ok("ok")
}

#[derive(Debug, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at handler boundary")]
pub struct TraverseParams {
    pub citation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC result DTO at handler boundary")]
pub struct CitersResult {
    pub citers: Vec<String>,
}

/// List the nodes that cite a citation (single backward `<-cite` walk).
///
/// # Errors
/// Returns `ErrorObjectOwned` on a malformed record id or DB failure.
pub async fn traverse(
    state: &AppState,
    p: TraverseParams,
) -> Result<CitersResult, ErrorObjectOwned> {
    let citation = parse_record(&p.citation)?;
    let citers = citation_traverse(&state.db, &citation)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(CitersResult {
        citers: citers.iter().map(|id| format!("{id:?}")).collect(),
    })
}

#[derive(Debug, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at handler boundary")]
pub struct RefreshParams {
    pub citation: String,
    pub delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC result DTO at handler boundary")]
pub struct RewardResult {
    pub rewarded: usize,
}

/// Flow RLAIF reward along the `cite` edges into a citation.
///
/// # Errors
/// Returns `ErrorObjectOwned` on a malformed record id or DB failure.
pub async fn refresh(
    state: &AppState,
    p: RefreshParams,
) -> Result<RewardResult, ErrorObjectOwned> {
    let citation = parse_record(&p.citation)?;
    let rewarded = citation_reward(&state.db, &citation, p.delta)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(RewardResult { rewarded })
}
