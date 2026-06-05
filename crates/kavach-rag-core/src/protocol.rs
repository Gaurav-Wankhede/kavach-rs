use serde::{Deserialize, Serialize};

use super::error::RagError;
use super::node::{NodeId, TreeNode};

/// A single request sent to the external summarizer (stdout line).
///
/// The summarizer (an LLM, an external script, or the assistant itself) reads
/// one request per line, emits one [`SummaryResponse`] per line on stdin.
/// Protocol is newline-delimited JSON — simple, language-agnostic, streamable.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[expect(clippy::exhaustive_structs, reason = "constructed cross-crate")]
pub struct SummaryRequest {
    pub node_id: NodeId,
    pub title: String,
    pub body: String,
}

/// A single response from the summarizer. `summary` fills `TreeNode::summary`;
/// `keywords` are merged into `TreeNode::keywords`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[expect(clippy::exhaustive_structs, reason = "constructed cross-crate")]
pub struct SummaryResponse {
    pub node_id: NodeId,
    pub summary: String,
    #[serde(default)]
    pub keywords: Vec<String>,
}

impl SummaryRequest {
    #[must_use]
    pub fn from_node(node: &TreeNode) -> Self {
        Self {
            node_id: node.id.clone(),
            title: node.title.clone(),
            body: node.body.clone(),
        }
    }

    /// Serialize to a newline-delimited JSON line.
    ///
    /// # Errors
    ///
    /// Returns `RagError` if JSON serialization fails.
    pub fn to_line(&self) -> Result<String, RagError> {
        serde_json::to_string(self).map_err(RagError::from)
    }
}

impl SummaryResponse {
    /// Deserialize from a newline-delimited JSON line.
    ///
    /// # Errors
    ///
    /// Returns `RagError` if JSON deserialization fails.
    pub fn from_line(line: &str) -> Result<Self, RagError> {
        serde_json::from_str(line).map_err(RagError::from)
    }
}

/// Walk `root` depth-first, apply summaries from `responses` to every matching
/// node in place. Responses with unknown ids are ignored (not an error — the
/// summarizer may batch multiple trees).
pub fn apply_summaries(root: &mut TreeNode, responses: &[SummaryResponse]) {
    for resp in responses {
        if let Some(node) = find_mut(root, &resp.node_id) {
            node.summary.clone_from(&resp.summary);
            for kw in &resp.keywords {
                if !node.keywords.iter().any(|existing| existing == kw) {
                    node.keywords.push(kw.clone());
                }
            }
        }
    }
}

fn find_mut<'a>(node: &'a mut TreeNode, id: &str) -> Option<&'a mut TreeNode> {
    if node.id == id {
        return Some(node);
    }
    for child in &mut node.children {
        if let Some(hit) = find_mut(child, id) {
            return Some(hit);
        }
    }
    None
}

/// Emit a [`SummaryRequest`] for every node in the tree that still needs a
/// summary (empty `summary` field). Leaves with empty bodies are skipped.
#[must_use]
pub fn pending_requests(root: &TreeNode) -> Vec<SummaryRequest> {
    let mut out: Vec<SummaryRequest> = Vec::new();
    collect(root, &mut out);
    out
}

fn collect(node: &TreeNode, out: &mut Vec<SummaryRequest>) {
    if node.summary.is_empty() && !node.body.is_empty() {
        out.push(SummaryRequest::from_node(node));
    }
    for child in &node.children {
        collect(child, out);
    }
}
