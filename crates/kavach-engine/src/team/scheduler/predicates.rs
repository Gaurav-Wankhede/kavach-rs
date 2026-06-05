//! Pure status/dependency predicates for the ready-set computation.
use std::collections::HashSet;

use kavach_surreal::graph::roadmap_dag::RoadmapDag;

/// A roadmap status the scheduler may dispatch.
pub(super) fn is_runnable_status(status: &str) -> bool {
    matches!(status, "todo" | "in_progress")
}

/// A roadmap status that satisfies a dependency edge.
pub(super) fn is_terminal_status(status: &str) -> bool {
    matches!(status, "done" | "verified")
}

/// All dependency edges into `node_id` resolve to a terminal (done/verified) node.
pub(super) fn deps_done(dag: &RoadmapDag, node_id: &str, done: &HashSet<&str>) -> bool {
    dag.edges
        .iter()
        .filter(|e| (e.rel == "depends_on" || e.rel == "blocks") && e.target == *node_id)
        .all(|e| done.contains(e.source.as_str()))
}
