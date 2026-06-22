// split: intentional - cohesive rag_tree RPC group (tree_list_labels, tree_list_refreshable)
// JSON-RPC method handlers for rag_tree storage.
// Used by kavach-engine rag_router gate (read) and session_start (refresh).
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::{RagTreeLabel, RagTreeRefreshable, rag_tree_list, rag_tree_list_refreshable};

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
