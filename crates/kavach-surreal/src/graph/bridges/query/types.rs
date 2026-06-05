use crate::graph::types::Entity;
use serde::{Deserialize, Serialize};
use surrealdb_types::SurrealValue;

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[non_exhaustive]
pub struct BridgeHit {
    pub concept: Entity,
    pub edge: String,
    pub src_table: String,
    pub src_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[non_exhaustive]
pub struct ProjectHit {
    pub edge: String,
    pub src_table: String,
    pub src_key: String,
    pub project_slug: String,
}

#[derive(SurrealValue)]
pub(super) struct ConceptsRow {
    pub entry_key: String,
    pub concepts: Vec<Entity>,
}

#[derive(SurrealValue)]
pub(super) struct ProjectsRow {
    pub table: String,
    pub key: String,
    pub slug: String,
}

pub(super) const BRIDGE_QUERY_LIMIT: i64 = 500;
