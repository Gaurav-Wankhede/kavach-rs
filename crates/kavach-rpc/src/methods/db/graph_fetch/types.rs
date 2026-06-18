//! Graph fetch types.

use serde::{Deserialize, Serialize};

const DEFAULT_LIMIT: usize = 200;

/// Fetch the entity graph for the Knowledge Graph view, optionally filtered to a
/// single `entity_type`. `limit` caps the node count the layout must position.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct GraphFetchParams {
    #[serde(default)]
    pub entity_type: Option<String>,
    #[serde(default = "default_limit_const")]
    pub limit: usize,
}

/// Field names mirror the kavach-app KG renderer DTO exactly.
///
/// The wire contract is shared, not redefined per-side. `total` is the unclamped
/// entity count; `nodes.len()` is what the layout actually placed (≤ `limit`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct GraphFetchResult {
    pub success: bool,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub total: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub rel: String,
}

const fn default_limit_const() -> usize {
    DEFAULT_LIMIT
}
