// RPC methods for L1->L0 bridge edges + cross-project queries.
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::{
    BridgeHit, ProjectHit, graph_bridge_to_concept, graph_concepts_for_project,
    graph_projects_for_concept,
};
use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateParams {
    pub src_table: String,
    pub src_key: String,
    pub edge: String,
    pub concept: String,
}

impl CreateParams {
    #[must_use]
    pub const fn new(src_table: String, src_key: String, edge: String, concept: String) -> Self {
        Self {
            src_table,
            src_key,
            edge,
            concept,
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdResult {
    pub id: String,
}

/// Create a bridge edge from a source table/key to a concept.
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub async fn create(state: &AppState, p: CreateParams) -> Result<IdResult, ErrorObjectOwned> {
    let id = graph_bridge_to_concept(&state.db, &p.src_table, &p.src_key, &p.edge, &p.concept)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(IdResult {
        id: format!("{id:?}"),
    })
}

#[non_exhaustive]
#[derive(Debug, Serialize, Deserialize)]
pub struct ConceptsForParams {
    pub project: String,
}

impl ConceptsForParams {
    #[must_use]
    pub const fn new(project: String) -> Self {
        Self { project }
    }
}

/// Get all bridge concepts for a project.
///
/// # Errors
///
/// Returns an error if the database query fails.
pub async fn concepts_for(
    state: &AppState,
    p: ConceptsForParams,
) -> Result<Vec<BridgeHit>, ErrorObjectOwned> {
    graph_concepts_for_project(&state.db, &p.project)
        .await
        .map_err(surreal_to_rpc)
}

#[non_exhaustive]
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectsForParams {
    pub concept: String,
}

impl ProjectsForParams {
    #[must_use]
    pub const fn new(concept: String) -> Self {
        Self { concept }
    }
}

/// Get all projects that bridge to a specific concept.
///
/// # Errors
///
/// Returns an error if the database query fails.
pub async fn projects_for(
    state: &AppState,
    p: ProjectsForParams,
) -> Result<Vec<ProjectHit>, ErrorObjectOwned> {
    graph_projects_for_concept(&state.db, &p.concept)
        .await
        .map_err(surreal_to_rpc)
}
