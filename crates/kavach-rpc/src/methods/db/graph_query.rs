// ALGO: Graph traversal + rendering
//! `db.graph_query` RPC method — entity search + edge lookup.

use super::util::or_str;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;

mod list;
mod named;
mod types;

pub use types::{GraphQueryParams, GraphQueryResult};

const DEFAULT_ETYPE: &str = "skill";

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when the underlying `SurrealDB` query fails.
pub async fn graph_query(
    ctx: &AppState,
    params: GraphQueryParams,
) -> Result<GraphQueryResult, ErrorObjectOwned> {
    match params.name.as_deref() {
        Some(n) => {
            let etype = or_str(params.entity_type.as_deref(), DEFAULT_ETYPE);
            named::graph_query_named(ctx, etype, n).await
        }
        None => list::graph_query_list(ctx, params.entity_type.as_deref(), params.limit).await,
    }
}
