//! Parallel DAG scheduler over a project's roadmap.
//!
//! hub: re-exports the public scheduler API; the value types, the `DagScheduler`
//! struct + its plan/dispatch impl, and the ready-set predicates live in
//! submodules.
mod dag_scheduler;
mod predicates;
mod reward_router;
mod roles;
mod types;

#[cfg(test)]
mod tests;

pub use dag_scheduler::DagScheduler;
pub use reward_router::RewardRouter;
pub use roles::{role_assignments, role_for_node, role_for_title, RolePool};
pub use types::{DispatchPlan, Spawner, SpawnerKind, TeamDispatchError};
