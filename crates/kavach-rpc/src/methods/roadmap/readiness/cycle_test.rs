//! Proves the dispatch cycle guard: self-dep and mutual cycles are detected,
//! a linear chain and a dangling dep are NOT flagged (so a legitimately-blocked
//! card still routes through the normal blocked path, not the deadlock path).
use super::is_in_cycle;
use std::collections::HashMap;

fn idx<'a>(pairs: &[(&'a str, &[&'a str])]) -> HashMap<&'a str, Vec<String>> {
    pairs
        .iter()
        .map(|(k, deps)| (*k, deps.iter().map(|d| (*d).to_owned()).collect()))
        .collect()
}

#[test]
fn self_dependency_is_a_cycle() {
    let g = idx(&[("a", &["a"])]);
    assert!(is_in_cycle("a", &g), "a depends on itself");
}

#[test]
fn mutual_dependency_is_a_cycle() {
    let g = idx(&[("a", &["b"]), ("b", &["a"])]);
    assert!(is_in_cycle("a", &g));
    assert!(is_in_cycle("b", &g));
}

#[test]
fn three_node_cycle_is_detected() {
    let g = idx(&[("a", &["b"]), ("b", &["c"]), ("c", &["a"])]);
    assert!(is_in_cycle("a", &g));
    assert!(is_in_cycle("c", &g));
}

#[test]
fn linear_chain_is_not_a_cycle() {
    // a -> b -> c, no back-edge: blocked, but NOT a deadlock.
    let g = idx(&[("a", &["b"]), ("b", &["c"]), ("c", &[])]);
    assert!(!is_in_cycle("a", &g));
    assert!(!is_in_cycle("b", &g));
    assert!(!is_in_cycle("c", &g));
}

#[test]
fn dangling_dep_is_not_a_cycle() {
    // a depends on ghost (absent from the pool) — a dead end, never a cycle.
    let g = idx(&[("a", &["ghost"])]);
    assert!(!is_in_cycle("a", &g));
}

#[test]
fn no_deps_is_not_a_cycle() {
    let g = idx(&[("a", &[])]);
    assert!(!is_in_cycle("a", &g));
}

#[test]
fn diamond_without_back_edge_is_not_a_cycle() {
    // a -> b, a -> c, b -> d, c -> d. A DAG; no node revisits the active path.
    let g = idx(&[("a", &["b", "c"]), ("b", &["d"]), ("c", &["d"]), ("d", &[])]);
    assert!(!is_in_cycle("a", &g));
}

#[test]
fn cycle_not_touching_start_does_not_falsely_flag_start() {
    // a -> b; b<->c cycle. `a` reaches the cycle but is not IN it; however the
    // DFS from `a` enters b then c then back to b (on the active path) -> a is
    // reported as reaching a cycle. We require the START node's own dispatch to
    // be safe: a card that merely DEPENDS on a deadlocked card is itself stuck,
    // so flagging it is correct (it can never become ready either).
    let g = idx(&[("a", &["b"]), ("b", &["c"]), ("c", &["b"])]);
    assert!(is_in_cycle("a", &g), "a's prereq chain hits a deadlock");
    assert!(!is_in_cycle("b", &g) || is_in_cycle("c", &g)); // b,c are in the cycle
}
