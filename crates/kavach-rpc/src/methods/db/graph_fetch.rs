// ALGO: Graph traversal + node limit enforcement
//! `db.graph_fetch` RPC method — fetch connected nodes and edges.

use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;

mod impl_;
mod types;

pub use types::{GraphEdge, GraphFetchParams, GraphFetchResult, GraphNode};

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when the underlying `SurrealDB` query fails.
pub async fn graph_fetch(
    ctx: &AppState,
    params: GraphFetchParams,
) -> Result<GraphFetchResult, ErrorObjectOwned> {
    impl_::graph_fetch_impl(ctx, params).await
}
