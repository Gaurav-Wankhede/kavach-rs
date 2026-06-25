// split: intentional - cohesive rag_tree RPC group (list/refreshable + persist/fetch blob path)
// JSON-RPC method handlers for rag_tree storage.
// Used by kavach-engine rag_router gate (read) and session_start (refresh).
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::{
    RagTreeLabel, RagTreeRefreshable, RagTreeRow, rag_tree_get, rag_tree_list,
    rag_tree_list_refreshable, rag_tree_upsert_with_dir,
};
use serde::Deserialize;

#[cfg(test)]
#[path = "rag_test.rs"]
mod tests;

/// Lists all RAG tree labels.
///
/// # Errors
///
/// Returns an error if the database query fails.
pub async fn tree_list_labels(state: &AppState) -> Result<Vec<RagTreeLabel>, ErrorObjectOwned> {
    rag_tree_list(&state.db).await.map_err(surreal_to_rpc)
}

/// Lists refreshable RAG trees.
///
/// # Errors
///
/// Returns an error if the database query fails.
pub async fn tree_list_refreshable(
    state: &AppState,
) -> Result<Vec<RagTreeRefreshable>, ErrorObjectOwned> {
    rag_tree_list_refreshable(&state.db)
        .await
        .map_err(surreal_to_rpc)
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct TreeGetParams {
    pub source: String,
}

/// Fetches one persisted `rag_tree` row (blob + hash) by source label.
///
/// # Errors
///
/// Returns an error if the database query fails.
pub async fn tree_get(
    state: &AppState,
    params: TreeGetParams,
) -> Result<Option<RagTreeRow>, ErrorObjectOwned> {
    rag_tree_get(&state.db, &params.source)
        .await
        .map_err(surreal_to_rpc)
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct TreeUpsertParams {
    pub source: String,
    pub built_at: String,
    pub tree_json: Vec<u8>,
    pub source_hash: String,
    pub source_dir: String,
}

/// Persists (upserts) a built `rag_tree` blob keyed by source label.
///
/// # Errors
///
/// Returns an error if the database upsert fails.
pub async fn tree_upsert(
    state: &AppState,
    params: TreeUpsertParams,
) -> Result<(), ErrorObjectOwned> {
    rag_tree_upsert_with_dir(
        &state.db,
        &params.source,
        &params.built_at,
        &params.tree_json,
        &params.source_hash,
        &params.source_dir,
    )
    .await
    .map_err(surreal_to_rpc)
}
