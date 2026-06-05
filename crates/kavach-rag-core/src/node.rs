use serde::{Deserialize, Serialize};

/// Stable identifier for a tree node. Strings rather than integers so the
/// offline builder can produce human-readable ids like "skills/rust".
pub type NodeId = String;

/// A single node in the RAG tree.
///
/// Leaf nodes carry the full retrievable payload (`body`). Internal nodes
/// carry only the `summary` used for tree traversal; their `body` is empty.
/// Keywords and file-pattern globs are matcher hints — not hard requirements.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed/matched cross-crate; non_exhaustive => E0639/E0004"
)]
pub struct TreeNode {
    pub id: NodeId,
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub file_patterns: Vec<String>,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub children: Vec<Self>,
}

impl TreeNode {
    pub fn new_leaf(
        id: impl Into<NodeId>,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            summary: String::new(),
            keywords: Vec::new(),
            file_patterns: Vec::new(),
            body: body.into(),
            children: Vec::new(),
        }
    }

    #[must_use]
    pub const fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}
