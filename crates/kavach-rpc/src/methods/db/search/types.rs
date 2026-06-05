//! `db.search` request and response types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct SearchParams {
    pub project: String,
    pub category: Option<String>,
    pub status: Option<String>,
    pub since: Option<String>,
    pub contains: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct SearchResult {
    pub entries: Vec<SearchHit>,
}

/// One matched entry, flattened to display strings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct SearchHit {
    pub category: String,
    pub key: String,
    pub title: String,
    pub status: String,
}
