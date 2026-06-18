//! Critical-path length over the dependency DAG — the Gap-1 priority signal.
//!
//! A ready node's critical-path length is the longest chain of dependents that
//! transitively wait on it. Dispatching the longest-chain node first minimises
//! the makespan: the bottleneck task starts as early as possible, so its tail
//! of dependents is not the thing that ends up gating the whole project.
use std::collections::HashMap;

use kavach_surreal::graph::roadmap_dag::RoadmapDag;

/// Map each node id to its critical-path length (count of nodes on the longest
/// dependency chain rooted at it, inclusive — a leaf is 1).
///
/// `topo_order` MUST be a valid topological order of the dependency edges
/// (prerequisites first), exactly as [`RoadmapDag::toposort_or_cycle`] returns.
/// We walk it in reverse so each node sees its already-computed successors.
/// Only `depends_on`/`blocks` edges count — the same dependency relation the
/// toposort and ready predicate use; `references`/`mentions` are not blocking.
///
/// REJECTED: memoised DFS (same O(n+e) but recursion depth = chain length risks
/// stack blowup at n<=2000; this iterative reverse-topo sweep is flat);
/// Bellman-Ford negated weights (O(n*e), wasteful when a topo order exists).
#[must_use]
pub(super) fn critical_path_lengths(
    dag: &RoadmapDag,
    topo_order: &[String],
) -> HashMap<String, u32> {
    // pass in reverse topological order: cp(n) = 1 + max(cp(succ)), cp(leaf)=1.
    // General longest-path is NP-hard (no optimal substructure); on a DAG the
    // topo order makes it LINEAR with no revisits.
    // YEAR: 2026 | SEARCHED: 2026-06
    // SOURCE: https://algs4.cs.princeton.edu/44sp/ (CPM = longest path in DAG)
    let mut successors: HashMap<&str, Vec<&str>> = HashMap::with_capacity(dag.nodes.len());
    for e in &dag.edges {
        if e.rel == "depends_on" || e.rel == "blocks" {
            successors
                .entry(e.source.as_str())
                .or_default()
                .push(e.target.as_str());
        }
    }

    let mut cp: HashMap<String, u32> = HashMap::with_capacity(topo_order.len());
    // Reverse topological order: every successor is resolved before its source.
    for id in topo_order.iter().rev() {
        let longest_succ = successors
            .get(id.as_str())
            .into_iter()
            .flatten()
            .filter_map(|s| cp.get(*s).copied())
            .max()
            .unwrap_or(0);
        cp.insert(id.clone(), longest_succ.saturating_add(1));
    }
    cp
}

/// Reorder `ready` so the longest-critical-path node comes first. Stable on
/// ties (equal cp length keeps the incoming topological order). `topo_order` is
/// the same order used to compute the lengths — passed through to avoid a
/// second toposort.
#[must_use]
pub(super) fn prioritize_by_critical_path(
    mut ready: Vec<String>,
    dag: &RoadmapDag,
    topo_order: &[String],
) -> Vec<String> {
    let cp = critical_path_lengths(dag, topo_order);
    ready.sort_by(|a, b| {
        let ca = cp.get(a).copied().unwrap_or(0);
        let cb = cp.get(b).copied().unwrap_or(0);
        cb.cmp(&ca)
    });
    ready
}

#[cfg(test)]
#[path = "critical_path_test.rs"]
mod tests;
