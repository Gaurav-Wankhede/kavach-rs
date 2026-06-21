//! `DagScheduler::plan` (pure ready-set computation) + `dispatch` (plan + spawn).
use std::collections::HashSet;

use kavach_surreal::graph::roadmap_dag::{RoadmapDag, TopoOrder};

use super::super::predicates::{deps_done, is_runnable_status, is_terminal_status};
use super::super::types::{DispatchPlan, Spawner, TeamDispatchError};
use super::DagScheduler;

/// The plan/dispatch behaviour, split into its own file from the constructors +
/// getters in the parent for the <100-LOC nano-file layout.
#[expect(
    clippy::multiple_inherent_impl,
    reason = "split impl across files for nano-file layout; one logical type, two source files"
)]
impl DagScheduler {
    /// Compute the dispatch plan for `dag` given how many teammates are already
    /// active. Pure over the DAG snapshot — no I/O, fully unit-testable.
    ///
    /// `READY`: a node is ready iff its status is runnable (`todo`/`in_progress`)
    /// AND every dependency edge into it resolves to a `done`/`verified` node.
    /// Ordered by the topological wavefront so prerequisites precede dependents.
    ///
    /// # Errors
    /// [`TeamDispatchError::Cycle`] when the DAG has a cycle (deadlock guard).
    pub fn plan(
        &self,
        dag: &RoadmapDag,
        active_teammates: usize,
    ) -> Result<DispatchPlan, TeamDispatchError> {
        // Deadlock guard FIRST: a cycle means some nodes can never be ready.
        let order = match dag.toposort_or_cycle() {
            TopoOrder::Ordered(order) => order,
            TopoOrder::Cycle(keys) => return Err(TeamDispatchError::Cycle(keys)),
        };

        let done: HashSet<&str> = dag
            .nodes
            .iter()
            .filter(|n| is_terminal_status(&n.entry_status))
            .map(|n| n.id.as_str())
            .collect();
        // Only roadmap units are dispatchable tasks; decision/research/pattern
        // rows live in the same DAG (they can satisfy deps) but are never work
        // a teammate executes. Filter dispatch to category == "roadmap".
        // nodes (roadmap_dag CAPACITY). Beats BTreeSet O(log n) / linear O(n) at
        // this scale. TIME: O(n+e) | YEAR: 2026
        let runnable: HashSet<&str> = dag
            .nodes
            .iter()
            .filter(|n| n.category == "roadmap" && is_runnable_status(&n.entry_status))
            .map(|n| n.id.as_str())
            .collect();

        let ready: Vec<String> = order
            .iter()
            .filter(|id| runnable.contains(id.as_str()))
            .filter(|id| deps_done(dag, id, &done))
            .cloned()
            .collect();
        // Gap 1: critical-path priority — longest downstream chain dispatches first.
        let ready = super::critical_path::prioritize_by_critical_path(ready, dag, &order);

        let free_slots = self.cap().saturating_sub(active_teammates);
        let batch = ready.iter().take(free_slots).cloned().collect();
        Ok(DispatchPlan {
            ready,
            batch,
            free_slots,
        })
    }

    /// Execute one dispatch tick: plan, then spawn the claimable batch through
    /// `spawner`. Returns the dispatched teammate names. Callers re-tick on the
    /// `DAG_WAKE` advisory emitted when a teammate completes.
    ///
    /// # Errors
    /// [`TeamDispatchError::Cycle`] on a cyclic DAG; [`TeamDispatchError::Engine`]
    /// when a spawn backend fails.
    pub fn dispatch<S: Spawner>(
        &self,
        dag: &RoadmapDag,
        active_teammates: usize,
        spawner: &S,
    ) -> Result<Vec<String>, TeamDispatchError> {
        let plan = self.plan(dag, active_teammates)?;
        let mut dispatched = Vec::with_capacity(plan.batch.len());
        for key in &plan.batch {
            let title = dag
                .nodes
                .iter()
                .find(|n| &n.id == key)
                .map_or("", |n| n.title.as_str());
            dispatched.push(spawner.spawn(key, title)?);
        }
        Ok(dispatched)
    }
}
