//! Graph query types.

use serde::{Deserialize, Serialize};

const DEFAULT_LIMIT: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct GraphQueryParams {
    pub entity_type: Option<String>,
    pub name: Option<String>,
    #[serde(default = "default_limit_const")]
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct GraphQueryResult {
    pub success: bool,
    pub lines: Vec<String>,
    pub total: usize,
    pub shown: usize,
    pub error: Option<String>,
}

const fn default_limit_const() -> usize {
    DEFAULT_LIMIT
}
