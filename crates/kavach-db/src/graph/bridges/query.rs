// Bridge query entry points: concepts_for_project, projects_for_concept.
// Const query tables in sibling modules.
pub mod consts;
pub mod fetch;
pub mod types;

pub use fetch::{concepts_for_project, projects_for_concept};
pub use types::{BridgeHit, ProjectHit};
