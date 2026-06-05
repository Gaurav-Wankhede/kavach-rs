//! Single-tick `dispatch` tests: spawn only the batch, mid-batch failure
//! propagation, and a cycle blocking dispatch (not just plan).
use std::cell::RefCell;

use super::super::TeamDispatchError;
use super::common::{CountSpawner, ScriptSpawner, node, scheduler};
use kavach_surreal::graph::roadmap_dag::RoadmapDag;

#[test]
fn dispatch_spawns_only_the_batch() {
    let dag = RoadmapDag {
        nodes: vec![node("a", "todo"), node("b", "todo"), node("c", "todo")],
        edges: vec![],
    };
    let sp = CountSpawner(RefCell::new(Vec::new()));
    let names = scheduler(2).dispatch(&dag, 0, &sp).expect("acyclic");
    assert_eq!(names.len(), 2);
    assert_eq!(sp.0.borrow().len(), 2);
}

#[test]
fn spawner_failure_mid_batch_propagates_and_stops() {
    // Three independent tasks; toposort preserves insertion order for
    // independent nodes, so ready order is [a,b,c]. Fail at index 1.
    let dag = RoadmapDag {
        nodes: vec![node("a", "todo"), node("b", "todo"), node("c", "todo")],
        edges: vec![],
    };
    let sp = ScriptSpawner::new(Some(1));
    let err = scheduler(8)
        .dispatch(&dag, 0, &sp)
        .expect_err("should fail on 2nd spawn");
    assert!(matches!(err, TeamDispatchError::Engine(_)));
    let log = sp.call_log();
    // Two spawns should have been attempted before failure.
    assert_eq!(log.len(), 2);
    assert!(!log[1].is_empty());
}

#[test]
fn cycle_blocks_dispatch_not_just_plan() {
    let dag = RoadmapDag {
        nodes: vec![node("a", "todo"), node("b", "todo")],
        edges: vec![super::common::dep("a", "b"), super::common::dep("b", "a")],
    };
    let sp = ScriptSpawner::new(None);
    let err = scheduler(8)
        .dispatch(&dag, 0, &sp)
        .expect_err("cycle should fail");
    assert!(matches!(err, TeamDispatchError::Cycle(_)));
    assert_eq!(sp.call_log().len(), 0);
}
