//! Graph fetch implementation.

use crate::error::internal;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;

use super::types::{GraphEdge, GraphFetchParams, GraphFetchResult, GraphNode};

pub(super) async fn graph_fetch_impl(
    ctx: &AppState,
    params: GraphFetchParams,
) -> Result<GraphFetchResult, ErrorObjectOwned> {
    let etype = params.entity_type.as_deref();
    let entities = kavach_surreal::graph_list_entities(&ctx.db, etype)
        .await
        .map_err(|e| internal(e.to_string()))?;
    let total = entities.len();

    // Cap to the layout budget, then key each node by its record id stringified
    // the SAME way `graph_list_edges_among` stringifies endpoints, so edges
    // resolve against the rendered node set.
    let mut node_ids = Vec::with_capacity(entities.len().min(params.limit));
    let mut nodes = Vec::with_capacity(entities.len().min(params.limit));
    for ent in entities.into_iter().take(params.limit) {
        let Some(rid) = ent.id else { continue };
        nodes.push(GraphNode {
            id: format!("{rid:?}"),
            label: ent.name,
            kind: ent.entity_type,
        });
        node_ids.push(rid);
    }

    let edges = kavach_surreal::graph_list_edges_among(&ctx.db, &node_ids)
        .await
        .map_err(|e| internal(e.to_string()))?
        .into_iter()
        .map(|e| GraphEdge {
            from: e.from,
            to: e.to,
            rel: e.rel_type,
        })
        .collect();

    Ok(GraphFetchResult {
        success: true,
        nodes,
        edges,
        total,
        error: None,
    })
}
