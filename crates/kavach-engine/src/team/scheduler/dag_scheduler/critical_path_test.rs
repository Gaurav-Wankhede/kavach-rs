use super::{critical_path_lengths, prioritize_by_critical_path};
use kavach_surreal::graph::roadmap_dag::{DagEdge, DagNode, RoadmapDag};

fn node(id: &str) -> DagNode {
    DagNode {
        id: id.to_owned(),
        entry_key: id.to_owned(),
        title: id.to_owned(),
        entry_status: "todo".to_owned(),
        category: "roadmap".to_owned(),
    }
}

fn dep(src: &str, tgt: &str) -> DagEdge {
    // src must finish before tgt: src --depends_on--> tgt in this DAG's convention
    // (toposort treats depends_on/blocks as the ordering edge).
    DagEdge {
        source: src.to_owned(),
        target: tgt.to_owned(),
        rel: "depends_on".to_owned(),
    }
}

/// Chain a -> b -> c: a has the longest downstream chain (cp=3), c is a leaf (1).
#[test]
fn cp_lengths_follow_the_chain() {
    let dag = RoadmapDag {
        nodes: vec![node("a"), node("b"), node("c")],
        edges: vec![dep("a", "b"), dep("b", "c")],
    };
    let order = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];
    let cp = critical_path_lengths(&dag, &order);
    assert_eq!(cp.get("a").copied(), Some(3));
    assert_eq!(cp.get("b").copied(), Some(2));
    assert_eq!(cp.get("c").copied(), Some(1));
}

/// A leaf node with no dependents has critical-path length 1.
#[test]
fn isolated_node_is_length_one() {
    let dag = RoadmapDag {
        nodes: vec![node("solo")],
        edges: vec![],
    };
    let cp = critical_path_lengths(&dag, &["solo".to_owned()]);
    assert_eq!(cp.get("solo").copied(), Some(1));
}

/// Two ready roots: the one heading the longer chain sorts first even when the
/// incoming order puts the shorter one ahead.
#[test]
fn prioritize_puts_longest_chain_first() {
    // long: l0 -> l1 -> l2 (cp 3). short: s0 (cp 1). Incoming order: short first.
    let dag = RoadmapDag {
        nodes: vec![node("s0"), node("l0"), node("l1"), node("l2")],
        edges: vec![dep("l0", "l1"), dep("l1", "l2")],
    };
    let order = vec![
        "s0".to_owned(),
        "l0".to_owned(),
        "l1".to_owned(),
        "l2".to_owned(),
    ];
    // Only the two roots are "ready" (deps satisfied); incoming order = short first.
    let ready = vec!["s0".to_owned(), "l0".to_owned()];
    let out = prioritize_by_critical_path(ready, &dag, &order);
    assert_eq!(out, vec!["l0".to_owned(), "s0".to_owned()]);
}

/// Equal critical-path lengths preserve the incoming (topological) order — the
/// sort is stable, so prerequisites still precede dependents at equal depth.
#[test]
fn equal_depth_keeps_input_order() {
    let dag = RoadmapDag {
        nodes: vec![node("x"), node("y")],
        edges: vec![],
    };
    let order = vec!["x".to_owned(), "y".to_owned()];
    let ready = vec!["x".to_owned(), "y".to_owned()];
    let out = prioritize_by_critical_path(ready, &dag, &order);
    assert_eq!(out, vec!["x".to_owned(), "y".to_owned()]);
}
