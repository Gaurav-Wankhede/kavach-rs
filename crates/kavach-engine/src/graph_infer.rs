// SEMANTIC GRAPH INFERENCER — derives concept-reference edges from row content.
//
// ONE rule only: row B's content mentions row A's entry_key (word-boundary
// match) -> emit `references` edge (B -> A). This is the only real concept
// relationship: prose explicitly naming another entry.
//
// NOT inferred here (intentionally rejected):
//   - title-token overlap        (statistical similarity, not a relation)
//   - same-prefix temporal chain (sibling ordering, not a concept link)
//   - same status cluster        (report data, not a relation)
//   - status-transition pairs    (audit data, not a relation)
//
// SOURCE: https://surrealdb.com/docs/learn/data-models/graph/overview
// SOURCE: https://doc.rust-lang.org/std/primitive.str.html#method.contains
mod infer;
mod scan;
mod types;

#[cfg(test)]
mod tests;

pub use infer::infer_relationships;
pub use types::{InferRow, InferredRel};
