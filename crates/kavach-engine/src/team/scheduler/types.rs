//! Scheduler value types: dispatch error, spawner backend kind + trait, and the
//! per-tick dispatch plan. The `DagScheduler` itself lives in `dag_scheduler`.
use crate::error::EngineError;

/// Why a dispatch tick could not proceed.
#[derive(Debug, thiserror::Error)]
#[expect(
    clippy::exhaustive_enums,
    reason = "exhaustively matched cross-crate in kavach-cli cmd/team.rs; non_exhaustive => E0004"
)]
pub enum TeamDispatchError {
    /// The dependency DAG contains a cycle — no node in it can ever become
    /// ready, so dispatch is rejected rather than deadlocking. Names the keys.
    #[error("dependency cycle (deadlock) among: {0:?}")]
    Cycle(Vec<String>),
    /// Underlying engine/store failure while resolving or claiming tasks.
    #[error(transparent)]
    Engine(#[from] EngineError),
}

/// Which spawn backend a tick uses for the ready batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "matched in kavach-cli cmd/team.rs --spawner flag; non_exhaustive => E0004"
)]
pub enum SpawnerKind {
    /// Native CC Agent Teams (TeammateIdle/TaskCompleted hooks already wired).
    CcTeams,
    /// Workflow-tool fan-out (no experimental flag; one pipeline per wavefront).
    Workflow,
}

/// Abstraction over how a claimed task becomes a running agent. Both impls are
/// reversible behind this one method (swap the impl, no caller churn).
pub trait Spawner {
    /// Spawn one agent for `task_key` titled `title`. Returns the dispatched
    /// teammate name (for `active_teammates` accounting).
    ///
    /// # Errors
    /// Propagates backend failures (team API / workflow launch).
    fn spawn(&self, task_key: &str, title: &str) -> Result<String, EngineError>;
}

/// One scheduler tick's decision: the topologically-ordered ready wavefront,
/// the slice actually claimable given free slots, and whether a cycle blocked it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct DispatchPlan {
    /// All ready task keys this tick, in dependency order (prereqs first).
    pub ready: Vec<String>,
    /// The prefix of `ready` that fits in the free concurrency slots.
    pub batch: Vec<String>,
    /// free = cap - `active_teammates` at tick time.
    pub free_slots: usize,
}
