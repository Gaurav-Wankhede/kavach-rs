// L1->L0 bridges — connect project-scoped roadmap/decision/research/pattern/
// app_spec rows to global L0 concepts via 4 bridge edges.
// Arch: decision/concept-kg-iter2-combined-arch.
pub mod create;
pub mod query;

pub use create::bridge_to_concept;
pub use query::{BridgeHit, ProjectHit, concepts_for_project, projects_for_concept};
