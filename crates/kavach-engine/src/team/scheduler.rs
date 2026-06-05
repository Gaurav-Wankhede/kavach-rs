//! Parallel DAG scheduler over a project's roadmap.
//!
//! hub: re-exports the public scheduler API; the value types, the `DagScheduler`
//! struct + its plan/dispatch impl, and the ready-set predicates live in
//! submodules.
mod dag_scheduler;
mod predicates;
mod types;

#[cfg(test)]
mod tests;

pub use dag_scheduler::DagScheduler;
pub use types::{DispatchPlan, Spawner, SpawnerKind, TeamDispatchError};
