//! Dependency-cycle detection for the dispatch readiness path.
//!
//! `deps_satisfied` answers "are all prereqs done?" — but a card whose declared
//! deps form a cycle (the degenerate case being `DEPENDS_ON: <self>`, the general
//! case a mutual `A->B->A`) can NEVER have all prereqs done, so every dispatch
//! selector filters it out silently and the loop reports a FALSE `[ALL_BLOCKED]`
//! clean-stop while runnable work sits unreachable. This module surfaces such a
//! card as a hard, named error instead of letting it vanish.
use super::dep_key::parse_declared_deps;
use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;
/// True iff `start` participates in a dependency cycle reachable through the
/// content-declared `DEPENDS_ON:` edges resolved against `by_key`.
///
/// Self-dependency (`A` declares `A`) is the boundary case and returns `true`.
/// Dep keys absent from `by_key` are dead-ends (cannot close a cycle), matching
/// the tolerance of `deps_satisfied`. O(n+e) iterative DFS; generic over the
/// map's hasher so callers can pass any `HashMap`.
#[must_use]
pub fn is_in_cycle<S: BuildHasher>(start: &str, by_key: &HashMap<&str, Vec<String>, S>) -> bool {
    // Iterative three-color DFS that drains each frame's dep iterator fully, so
    // every dep of a multi-dep node is explored (a naive "first child then break"
    // misses a cycle reachable only through a later sibling — the boundary
    // loophole). A node already on the active path (or `start` itself) closing a
    // back-edge proves a cycle reachable from `start`.
    let empty: Vec<String> = Vec::new();
    let mut stack: Vec<(&str, std::slice::Iter<'_, String>)> =
        vec![(start, by_key.get(start).unwrap_or(&empty).iter())];
    let mut on_path: HashSet<&str> = HashSet::new();
    let mut done: HashSet<&str> = HashSet::new();
    on_path.insert(start);
    while let Some((node, deps)) = stack.last_mut() {
        if let Some(dep) = deps.next() {
            let d = dep.as_str();
            if d == start || on_path.contains(d) {
                return true; // back-edge into the active path => cycle
            }
            if !done.contains(d)
                && let Some(child_deps) = by_key.get(d)
            {
                on_path.insert(d);
                stack.push((d, child_deps.iter()));
            }
        } else {
            // frame fully explored: leaves the active path, marked black.
            on_path.remove(node);
            done.insert(node);
            stack.pop();
        }
    }
    false
}
/// Index every entry's declared deps once, keyed by `entry_key`, for repeated
/// [`is_in_cycle`] queries over the same pool.
#[must_use]
pub fn dep_index(pool: &[kavach_surreal::MemoryEntry]) -> HashMap<&str, Vec<String>> {
    pool.iter()
        .map(|e| (e.entry_key.as_str(), parse_declared_deps(&e.content)))
        .collect()
}
#[cfg(test)]
#[path = "cycle_test.rs"]
mod tests;
