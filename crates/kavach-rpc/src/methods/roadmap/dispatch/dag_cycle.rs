//! `depends_on` cycle detection for dispatch (E2).
//!
//! A `DEPENDS_ON:` cycle (A→B→A) makes `deps_satisfied` false for EVERY node in
//! the cycle, so `pick_in_lane` silently returns nothing — the loop stalls with
//! no signal ("picks neither"). This detector runs a DFS back-edge search over
//! the declared-dep adjacency and, on the first cycle found, returns the keys on
//! it so the caller can emit a `[DAG_CYCLE]` allow-stop NAMING them, instead of
//! an invisible stall. Pure over the entry slice — no DB, no I/O.
use super::super::readiness::parse_declared_deps;
use kavach_surreal::MemoryEntry;
use std::collections::HashMap;
/// DFS visit colour for back-edge cycle detection.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mark {
    /// On the current DFS stack — a re-visit here is a back edge = cycle.
    InProgress,
    /// Fully explored, no cycle through it — never revisited.
    Done,
}
/// Detect ONE `depends_on` cycle among `entries`. Returns the keys on the cycle
/// (in traversal order, cycle-start repeated implicitly by the caller's message)
/// or `None` when the declared-dep graph is acyclic.
///
/// Only edges to keys that EXIST in `entries` are followed — a dangling dep is a
/// missing node, not a cycle, and the dep-key parser already drops prose tokens.
#[must_use]
pub(super) fn detect_cycle(entries: &[MemoryEntry]) -> Option<Vec<String>> {
    // adjacency: key -> its declared deps that also exist as nodes here.
    let present: std::collections::HashSet<&str> =
        entries.iter().map(|e| e.entry_key.as_str()).collect();
    let adj: HashMap<&str, Vec<String>> = entries
        .iter()
        .map(|e| {
            let deps = parse_declared_deps(&e.content)
                .into_iter()
                .filter(|d| present.contains(d.as_str()))
                .collect();
            (e.entry_key.as_str(), deps)
        })
        .collect();
    let mut mark: HashMap<&str, Mark> = HashMap::new();
    let mut stack: Vec<String> = Vec::new();
    for e in entries {
        if let Some(cycle) = dfs(e.entry_key.as_str(), &adj, &mut mark, &mut stack) {
            return Some(cycle);
        }
    }
    None
}
/// Recursive DFS: returns the cycle path (from the re-entered node to the current
/// node) on the first back edge, else `None`. `stack` holds the current path keys
/// so the cycle slice can be reported by name. The `'a` lifetime ties the visit
/// marks to the adjacency's owned keys so they outlive the recursion.
fn dfs<'a>(
    node: &'a str,
    adj: &HashMap<&'a str, Vec<String>>,
    mark: &mut HashMap<&'a str, Mark>,
    stack: &mut Vec<String>,
) -> Option<Vec<String>> {
    match mark.get(node) {
        Some(Mark::Done) => return None,
        Some(Mark::InProgress) => {
            // Back edge: the cycle is the stack slice from `node` to the top.
            // `position` is Some (node is on the stack, it is InProgress), and
            // `get(start..)` is panic-free even if that invariant ever weakened.
            let start = stack.iter().position(|k| k == node).unwrap_or(0);
            return stack.get(start..).map(<[String]>::to_vec);
        }
        None => {}
    }
    mark.insert(node, Mark::InProgress);
    stack.push(node.to_owned());
    if let Some(deps) = adj.get(node) {
        for dep in deps {
            // Re-key to the adjacency's owned `&'a str` so the borrow lives long
            // enough; a dep absent as a node was filtered out at adjacency build.
            if let Some((dep_key, _)) = adj.get_key_value(dep.as_str())
                && let Some(cycle) = dfs(dep_key, adj, mark, stack)
            {
                return Some(cycle);
            }
        }
    }
    stack.pop();
    mark.insert(node, Mark::Done);
    None
}
/// Format the `[DAG_CYCLE]` allow-stop message naming the cycle keys.
#[must_use]
pub(super) fn cycle_message(keys: &[String]) -> String {
    format!(
        "[DAG_CYCLE] dispatch stalled: a DEPENDS_ON cycle among [{}] makes every \
         card in it un-runnable (each waits on the next). Break the cycle — drop \
         or redirect one DEPENDS_ON edge — then dispatch resumes.",
        keys.join(" -> ")
    )
}
#[cfg(test)]
#[path = "dag_cycle_test.rs"]
#[path = "dag_cycle_test.rs"]
mod tests;
