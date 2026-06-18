//! Pure status/dependency predicates for the ready-set computation.
use std::collections::HashSet;

use kavach_surreal::graph::roadmap_dag::RoadmapDag;

/// A roadmap status the scheduler may dispatch. Parsed through the typed
/// `MemoryStatus` boundary; a non-canonical value fail-closes to non-runnable.
pub(super) fn is_runnable_status(status: &str) -> bool {
    status
        .parse::<kavach_types::MemoryStatus>()
        .is_ok_and(kavach_types::MemoryStatus::is_runnable)
}

/// A roadmap status that satisfies a dependency edge. Parsed through the typed
/// `MemoryStatus` boundary; a non-canonical value fail-closes to non-terminal.
pub(super) fn is_terminal_status(status: &str) -> bool {
    status
        .parse::<kavach_types::MemoryStatus>()
        .is_ok_and(kavach_types::MemoryStatus::is_complete)
}

/// All dependency edges into `node_id` resolve to a terminal (done/verified) node.
pub(super) fn deps_done(dag: &RoadmapDag, node_id: &str, done: &HashSet<&str>) -> bool {
    dag.edges
        .iter()
        .filter(|e| (e.rel == "depends_on" || e.rel == "blocks") && e.target == *node_id)
        .all(|e| done.contains(e.source.as_str()))
}
