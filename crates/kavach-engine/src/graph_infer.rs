// Derives concept-reference edges from row content (word-boundary matches only).
// See decision.engine.graph-infer-semantic-rules for scope + carve-outs.
mod infer;
mod scan;
mod types;
#[cfg(test)]
#[path = "graph_infer_test.rs"]
#[path = "graph_infer_test.rs"]
mod tests;
pub use infer::infer_relationships;
pub use types::{InferRow, InferredRel};
