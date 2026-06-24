// split: intentional - cohesive project lookup RPC group (find/get/list/ancestry)
// JSON-RPC method handlers for project lookups.
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::Project;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct AncestryParams {
    pub slug: String,
}

/// Get ancestry chain for a project by slug.
///
/// # Errors
///
/// Returns an error if the database query fails or if the project cannot be found.
pub async fn ancestry(
    state: &AppState,
    params: AncestryParams,
) -> Result<Vec<Project>, ErrorObjectOwned> {
    let Some(proj) = kavach_surreal::project_get_by_slug(&state.db, &params.slug)
        .await
        .map_err(surreal_to_rpc)?
    else {
        return Ok(Vec::new());
    };
    let Some(id) = proj.id.clone() else {
        return Ok(Vec::new());
    };
    kavach_surreal::project_get_ancestry(&state.db, &id)
        .await
        .map_err(surreal_to_rpc)
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct FindByPathParams {
    pub path: String,
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct GetBySlugParams {
    pub slug: String,
}

/// Find a project by its path.
///
/// # Errors
///
/// Returns an error if the database query fails.
pub async fn find_by_path(
    state: &AppState,
    params: FindByPathParams,
) -> Result<Option<Project>, ErrorObjectOwned> {
    kavach_surreal::project_find_by_path(&state.db, &params.path)
        .await
        .map_err(surreal_to_rpc)
}

/// Get a project by its slug.
///
/// # Errors
///
/// Returns an error if the database query fails.
pub async fn get_by_slug(
    state: &AppState,
    params: GetBySlugParams,
) -> Result<Option<Project>, ErrorObjectOwned> {
    kavach_surreal::project_get_by_slug(&state.db, &params.slug)
        .await
        .map_err(surreal_to_rpc)
}

/// List all projects.
///
/// # Errors
///
/// Returns an error if the database query fails.

