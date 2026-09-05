use crate::graph::types::Entity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct BridgeHit {
    pub concept: Entity,
    pub edge: String,
    pub src_table: String,
    pub src_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProjectHit {
    pub edge: String,
    pub src_table: String,
    pub src_key: String,
    pub project_slug: String,
}

pub(super) const BRIDGE_QUERY_LIMIT: i64 = 500;
