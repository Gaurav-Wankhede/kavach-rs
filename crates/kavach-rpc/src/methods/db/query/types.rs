// ALGO: Linear scan
// DATA_STRUCTURE: Vec
//! Query RPC request and response types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct QueryParams {
    pub project: String,
    pub category: Option<String>,
    #[serde(default)]
    pub all: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct QueryResult {
    pub entries: Vec<QueryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct QueryEntry {
    pub key: String,
    pub title: String,
    pub category: String,
    pub status: String,
    pub content: Option<String>,
    pub access_count: i64,
}
