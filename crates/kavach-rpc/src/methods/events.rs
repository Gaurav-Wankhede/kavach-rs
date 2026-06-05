// JSON-RPC method handler for event.append.
// Used by kavach-engine event_log helpers to record gate-emitted events
// without spawning tokio per call.
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct AppendParams {
    pub event_type: String,
    pub source: String,
    pub project: Option<String>,
    pub payload: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct AppendResult {
    pub id: String,
}

/// Appends an event to the event log.
///
/// # Errors
///
/// Returns an RPC error if the project lookup fails or the event append fails.
pub async fn append(
    state: &AppState,
    params: AppendParams,
) -> Result<Option<AppendResult>, ErrorObjectOwned> {
    let project_id = match params.project.as_deref() {
        Some(slug) if !slug.is_empty() => {
            let proj = kavach_surreal::project_get_by_slug(&state.db, slug)
                .await
                .map_err(surreal_to_rpc)?;
            proj.and_then(|p| p.id)
        }
        Some(_) | None => None,
    };
    let id = kavach_surreal::append_event(
        &state.db,
        &params.event_type,
        &params.source,
        project_id,
        params.payload.as_deref(),
    )
    .await
    .map_err(surreal_to_rpc)?;
    Ok(Some(AppendResult {
        id: format!("{id:?}"),
    }))
}
