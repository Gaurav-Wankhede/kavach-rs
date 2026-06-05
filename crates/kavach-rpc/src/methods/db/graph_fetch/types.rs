// ALGO: DTO definitions
//! Graph fetch types.

use serde::{Deserialize, Serialize};

const DEFAULT_NODE_LIMIT: usize = 100;
const DEFAULT_EDGE_LIMIT: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct GraphFetchParams {
    pub root_id: String,
    #[serde(default = "default_node_limit_const")]
    pub node_limit: usize,
    #[serde(default = "default_edge_limit_const")]
    pub edge_limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct GraphFetchResult {
    pub success: bool,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub node_count: usize,
    pub edge_count: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub node_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub rel_type: String,
}

const fn default_node_limit_const() -> usize {
    DEFAULT_NODE_LIMIT
}

const fn default_edge_limit_const() -> usize {
    DEFAULT_EDGE_LIMIT
}
