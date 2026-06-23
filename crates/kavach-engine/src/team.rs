//! DAG-aware parallel Team auto-dispatch.
//!
//! The roadmap rows in kavach-db form a dependency DAG (`->depends_on->` /
//! `->blocks->` edges). This module fans CC Team agents out across that DAG:
//! independent tasks dispatch concurrently; a task blocked on another stays
//! out of the ready-set until its prerequisite reaches `done`/`verified`.
//!
//! The DAG store ([`kavach_surreal::graph::roadmap_dag`]) and its cycle guard
//! ([`RoadmapDag::toposort_or_cycle`]) already exist; this layer is the
//! parallel scheduler over them.

mod scheduler;
mod vendor;

pub use scheduler::{
    role_assignments, role_for_node, role_for_title, DagScheduler, DispatchPlan, RewardRouter,
    RolePool, Spawner, SpawnerKind, TeamDispatchError,
};
pub use vendor::{AgentRole, CommandBackend, VendorBackend, VendorOutput, VendorRequest};
