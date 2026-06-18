use std::collections::HashMap;

use super::node::{NodeId, TreeNode};
use super::query::Query;
use super::score::{Score, score_node, score_node_with_boost};
use super::tree::RagTree;

#[derive(Debug)]
pub struct Matcher<'a> {
    tree: &'a RagTree,
    top_k: usize,
    graph_boosts: Option<HashMap<String, u32>>,
}

/// A single match result: node id, title (for user-facing messages), and
/// computed score so gates can threshold / rank.
#[derive(Debug, Clone)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed/matched cross-crate; non_exhaustive => E0639"
)]
pub struct MatchResult {
    pub node_id: NodeId,
    pub title: String,
    pub score: Score,
}

impl<'a> Matcher<'a> {
    #[must_use]
    pub const fn new(tree: &'a RagTree) -> Self {
        Self {
            tree,
            top_k: 5,
            graph_boosts: None,
        }
    }

    #[must_use]
    pub fn with_top_k(mut self, top_k: usize) -> Self {
        self.top_k = top_k.max(1);
        self
    }

    /// Attach pre-computed graph boosts (`skill_name` → `boost_weight`).
    /// Built via `rag::graph_boost::compute_graph_boosts`.
    #[must_use]
    pub fn with_graph_boosts(mut self, boosts: HashMap<String, u32>) -> Self {
        self.graph_boosts = Some(boosts);
        self
    }

    #[must_use]
    pub fn run(&self, query: &Query) -> Vec<MatchResult> {
        let mut hits: Vec<MatchResult> = Vec::new();
        walk(
            &self.tree.root,
            query,
            self.graph_boosts.as_ref(),
            &mut hits,
        );
        hits.sort_by_key(|h| std::cmp::Reverse(h.score));
        hits.truncate(self.top_k);
        hits
    }
}

fn walk(
    node: &TreeNode,
    query: &Query,
    boosts: Option<&HashMap<String, u32>>,
    out: &mut Vec<MatchResult>,
) {
    let score = boosts.map_or_else(
        || score_node(node, query),
        |map| {
            let skill_name = extract_skill_name(&node.id);
            let boost = map.get(skill_name).copied().unwrap_or(0);
            score_node_with_boost(node, query, boost)
        },
    );
    if score.is_nonzero() {
        out.push(MatchResult {
            node_id: node.id.clone(),
            title: node.title.clone(),
            score,
        });
    }
    for child in &node.children {
        walk(child, query, boosts, out);
    }
}

/// Extract skill name from node id like "skills/rust/SKILL.md" → "rust"
fn extract_skill_name(node_id: &str) -> &str {
    let stripped = node_id.trim_end_matches("/SKILL.md");
    match stripped.rsplit('/').next() {
        Some(name) if !name.is_empty() => name,
        Some(_) | None => node_id,
    }
}
