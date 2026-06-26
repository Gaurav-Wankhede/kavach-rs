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

pub use scheduler::{DagScheduler, DispatchPlan, Spawner, SpawnerKind, TeamDispatchError};
