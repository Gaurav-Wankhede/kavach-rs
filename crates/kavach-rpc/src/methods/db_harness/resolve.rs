// Shared project-slug -> RecordId resolver for the harness-loop RPC methods.
use crate::error::internal;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use surrealdb_types::RecordId;

/// Resolve a project slug to its `RecordId`, mirroring `db::resolve_project_id`.
///
/// # Errors
/// Returns an RPC `ErrorObjectOwned` when the project is absent or has no id.
pub(super) async fn project_id(state: &AppState, slug: &str) -> Result<RecordId, ErrorObjectOwned> {
    let project = kavach_surreal::project_get_by_slug(&state.db, slug)
        .await
        .map_err(|e| internal(e.to_string()))?;
    match project {
        Some(p) => {
            p.id.map_or_else(|| Err(internal(format!("project has no id: {slug}"))), Ok)
        }
        None => Err(internal(format!("project not found: {slug}"))),
    }
}
