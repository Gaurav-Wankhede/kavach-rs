// ALGO: Graph traversal + node limit enforcement
//! Graph fetch implementation.

use crate::error::internal;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;

use super::types::{GraphEdge, GraphFetchParams, GraphFetchResult, GraphNode};

pub(super) async fn graph_fetch_impl(
    ctx: &AppState,
    params: GraphFetchParams,
) -> Result<GraphFetchResult, ErrorObjectOwned> {
    use std::fmt::Write as _;

    let root_rid = surrealdb_types::RecordId::parse_simple(&params.root_id)
        .map_err(|e| internal(format!("invalid root_id: {e}")))?;
    let rows = kavach_surreal::graph_get_related(&ctx.db, &root_rid, params.edge_limit)
        .await
        .map_err(|e| internal(e.to_string()))?;
    let mut nodes = Vec::with_capacity(params.node_limit.saturating_add(1));
    nodes.push(GraphNode {
        id: params.root_id.clone(),
        label: "root".to_owned(),
        node_type: "unknown".to_owned(),
    });
    let mut edges = Vec::with_capacity(params.node_limit);
    for (idx, r) in rows.iter().enumerate() {
        if idx >= params.node_limit {
            break;
        }
        let node_id = r.target.id.as_ref().map_or_else(
            || {
                let mut s = String::with_capacity(16);
                #[expect(clippy::expect_used, reason = "write to String never fails")]
                write!(s, "node_{idx}").expect("write to String never fails");
                s
            },
            |id| {
                let mut s = String::with_capacity(32);
                #[expect(clippy::expect_used, reason = "write to String never fails")]
                write!(s, "{id:?}").expect("write to String never fails");
                s
            },
        );
        nodes.push(GraphNode {
            id: node_id.clone(),
            label: r.target.name.clone(),
            node_type: r.target.entity_type.clone(),
        });
        edges.push(GraphEdge {
            source: params.root_id.clone(),
            target: node_id,
            rel_type: r.rel_type.clone(),
        });
    }
    let node_count = nodes.len();
    let edge_count = edges.len();
    Ok(GraphFetchResult {
        success: true,
        nodes,
        edges,
        node_count,
        edge_count,
        error: None,
    })
}
