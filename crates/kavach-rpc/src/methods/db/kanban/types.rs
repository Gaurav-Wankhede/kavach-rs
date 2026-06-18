//! Kanban request/response types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct KanbanParams {
    pub project: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    pub status: Option<String>,
    pub key: Option<String>,
}

impl KanbanParams {
    #[must_use]
    pub const fn new(
        project: String,
        limit: usize,
        status: Option<String>,
        key: Option<String>,
    ) -> Self {
        Self {
            project,
            limit,
            status,
            key,
        }
    }
}

const fn default_limit() -> usize {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct KanbanItem {
    pub key: String,
    pub title: String,
    pub status: String,
    pub category: String,
    #[serde(default)]
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct KanbanResult {
    pub items: Vec<KanbanItem>,
    pub counts: KanbanCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub struct KanbanCounts {
    pub todo: usize,
    pub in_progress: usize,
    pub done: usize,
    pub verified: usize,
}
