//! Single-tick `plan` tests: ready-set, dependency chains, cycle rejection,
//! and free-slot clamping.
use std::collections::HashSet;

use super::super::TeamDispatchError;
use super::common::{dep, node, scheduler};
use kavach_surreal::graph::roadmap_dag::RoadmapDag;

#[test]
fn two_independent_tasks_both_ready() {
    let dag = RoadmapDag {
        nodes: vec![node("a", "todo"), node("b", "todo")],
        edges: vec![],
    };
    let plan = scheduler(8).plan(&dag, 0).expect("acyclic");
    assert_eq!(plan.ready.len(), 2);
    assert_eq!(plan.batch.len(), 2);
}

#[test]
fn chain_blocks_dependent_until_prereq_done() {
    // a -> b -> c ; only a ready while a is todo
    let edges = vec![dep("a", "b"), dep("b", "c")];
    let dag = RoadmapDag {
        nodes: vec![node("a", "todo"), node("b", "todo"), node("c", "todo")],
        edges: edges.clone(),
    };
    let plan = scheduler(8).plan(&dag, 0).expect("acyclic");
    assert_eq!(plan.ready, vec!["a".to_owned()]);

    // a done -> b becomes ready, c still blocked
    let dag2 = RoadmapDag {
        nodes: vec![node("a", "done"), node("b", "todo"), node("c", "todo")],
        edges,
    };
    let plan2 = scheduler(8).plan(&dag2, 0).expect("acyclic");
    assert_eq!(plan2.ready, vec!["b".to_owned()]);
}

#[test]
fn cycle_is_rejected_with_keys() {
    // a -> b -> a
    let dag = RoadmapDag {
        nodes: vec![node("a", "todo"), node("b", "todo")],
        edges: vec![dep("a", "b"), dep("b", "a")],
    };
    let err = scheduler(8).plan(&dag, 0).expect_err("cycle");
    match err {
        TeamDispatchError::Cycle(keys) => {
            assert_eq!(keys.iter().collect::<HashSet<_>>().len(), 2);
        }
        TeamDispatchError::Engine(_) => panic!("expected Cycle"),
    }
}

#[test]
fn free_slots_clamp_batch_below_ready() {
    let dag = RoadmapDag {
        nodes: vec![node("a", "todo"), node("b", "todo"), node("c", "todo")],
        edges: vec![],
    };
    // cap 8, 6 active -> 2 free; 3 ready but only 2 dispatched this tick
    let plan = scheduler(8).plan(&dag, 6).expect("acyclic");
    assert_eq!(plan.ready.len(), 3);
    assert_eq!(plan.free_slots, 2);
    assert_eq!(plan.batch.len(), 2);
}
