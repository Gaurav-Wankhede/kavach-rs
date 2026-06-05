//! kavach-rag-core — pure-fn RAG types extracted from kavach-db.
//!
//! Vectorless, reasoning-based RAG index for kavach gates. Replaces static
//! skill-priority tables with a hierarchical tree whose nodes carry summaries
//! and keyword hints. Hook hot paths traverse the tree via pure-Rust scoring —
//! no embeddings, no LLM, no network calls at runtime.
//!
//! Zero storage coupling: this crate has no surrealdb dependency.
//! Graph-boost (which needs a Connection) stays in kavach-surreal.

pub mod error;
pub mod matcher;
pub mod node;
pub mod protocol;
pub mod query;
pub mod scanner;
pub mod score;
pub mod tree;
pub mod walker;

pub use error::RagError;
pub use matcher::{MatchResult, Matcher};
pub use node::{NodeId, TreeNode};
pub use protocol::{SummaryRequest, SummaryResponse, apply_summaries, pending_requests};
pub use query::Query;
pub use scanner::{ScannedDoc, scan_dir};
pub use score::{Score, score_node, score_node_with_boost};
pub use tree::RagTree;
pub use walker::{build_trees_from_dir, from_markdown};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
