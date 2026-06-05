use serde::{Deserialize, Serialize};

use super::error::RagError;
use super::node::TreeNode;

/// Root container for a persisted RAG tree.
///
/// Built offline by `kavach rag build` (Phase B), consumed at runtime by
/// `Matcher`. The `version` field lets us evolve the schema without breaking
/// previously-built trees.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct RagTree {
    pub version: u32,
    pub source: String,
    pub root: TreeNode,
}

impl RagTree {
    pub const CURRENT_VERSION: u32 = 1;

    pub fn new(source: impl Into<String>, root: TreeNode) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            source: source.into(),
            root,
        }
    }

    /// Deserialize a RAG tree from JSON and validate it.
    ///
    /// # Errors
    ///
    /// Returns `RagError` if JSON parsing fails or tree validation fails.
    pub fn from_json(json: &str) -> Result<Self, RagError> {
        let tree: Self = serde_json::from_str(json)?;
        tree.validate()?;
        Ok(tree)
    }

    /// Serialize the RAG tree to JSON.
    ///
    /// # Errors
    ///
    /// Returns `RagError` if serialization fails.
    pub fn to_json(&self) -> Result<String, RagError> {
        serde_json::to_string(self).map_err(RagError::from)
    }

    /// Serialize the RAG tree to pretty-printed JSON.
    ///
    /// # Errors
    ///
    /// Returns `RagError` if serialization fails.
    pub fn to_json_pretty(&self) -> Result<String, RagError> {
        serde_json::to_string_pretty(self).map_err(RagError::from)
    }

    /// Maximum tree depth. Trees deeper than this are rejected at `validate()`
    /// time so the recursive walkers (find / `Matcher::run`) can never recurse
    /// to stack exhaustion on attacker-supplied input.
    pub const MAX_DEPTH: usize = 512;

    /// Reject trees with schema drift, unsupported versions, or unbounded
    /// nesting early so runtime gates never see malformed/hostile state.
    ///
    /// # Errors
    ///
    /// Returns `RagError` if the tree version is unsupported, the root node has an empty ID,
    /// or the tree exceeds the maximum depth limit.
    pub fn validate(&self) -> Result<(), RagError> {
        if self.version != Self::CURRENT_VERSION {
            return Err(RagError::Invalid(format!(
                "unsupported version {}, expected {}",
                self.version,
                Self::CURRENT_VERSION
            )));
        }
        if self.root.id.is_empty() {
            return Err(RagError::Invalid("root node has empty id".into()));
        }
        // FIX: [stack_overflow CWE-674] tree.rs:46
        // WHY5: any recursive traversal over externally-sourced data MUST be
        //       depth-bounded; unbounded recursion on untrusted input is a
        //       DoS. from_json() calls validate() — the universal chokepoint
        //       every consumer (find, Matcher::run) passes through, so a
        //       bound here neutralizes both recursive walkers.
        // ROOT_CAUSE: validate() was incomplete (no depth check); walkers
        //             recurse per child with no bound.
        // RESEARCH: cwe.mitre.org/data/definitions/674.html; OWASP A04:2025.
        // Iterative (explicit stack) depth scan so the CHECK itself cannot
        // stack-overflow on the very input it guards against.
        let mut stack: Vec<(&TreeNode, usize)> = vec![(&self.root, 1)];
        while let Some((node, depth)) = stack.pop() {
            if depth > Self::MAX_DEPTH {
                return Err(RagError::Invalid(format!(
                    "tree depth exceeds maximum {} (possible resource-exhaustion input)",
                    Self::MAX_DEPTH
                )));
            }
            for child in &node.children {
                stack.push((child, depth.saturating_add(1)));
            }
        }
        Ok(())
    }

    /// Find a node by ID in the tree.
    ///
    /// # Errors
    ///
    /// Returns `RagError::NodeNotFound` if no node with the given ID exists in the tree.
    pub fn find(&self, id: &str) -> Result<&TreeNode, RagError> {
        walk(&self.root, id).ok_or_else(|| RagError::NodeNotFound(id.into()))
    }
}

fn walk<'a>(node: &'a TreeNode, id: &str) -> Option<&'a TreeNode> {
    if node.id == id {
        return Some(node);
    }
    for child in &node.children {
        if let Some(hit) = walk(child, id) {
            return Some(hit);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::TreeNode;

    /// Build a linear chain `depth` nodes deep, iteratively (so the test
    /// helper itself can't stack-overflow building the hostile input).
    fn chain(depth: usize) -> TreeNode {
        let mut node = TreeNode::new_leaf("n0", "t", "b");
        for i in 1..depth {
            let mut parent = TreeNode::new_leaf(format!("n{i}"), "t", "b");
            parent.children.push(node);
            node = parent;
        }
        node
    }

    #[test]
    fn validate_rejects_overdeep_tree_without_stack_overflow() {
        // Deeper than MAX_DEPTH → must return Err, NOT crash.
        let root = chain(RagTree::MAX_DEPTH + 50);
        let tree = RagTree::new("hostile", root);
        let err = tree
            .validate()
            .expect_err("over-deep tree must be rejected");
        if let RagError::Invalid(m) = err {
            assert!(m.contains("depth exceeds maximum"));
        } else {
            panic!("expected Invalid depth error, got {err:?}");
        }
    }

    #[test]
    fn validate_accepts_tree_within_depth_bound() {
        let root = chain(10);
        let tree = RagTree::new("ok", root);
        assert!(tree.validate().is_ok(), "shallow tree must validate");
    }
}
