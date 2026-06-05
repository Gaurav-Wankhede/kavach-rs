//! Multi-tick dispatch dynamics: dependency-ordered chains, cap=1 serialization,
//! and active-teammate free-slot accounting.
use super::common::{ScriptSpawner, dep, node, scheduler};
use kavach_surreal::graph::roadmap_dag::RoadmapDag;

#[test]
fn multi_tick_chain_dispatches_in_dependency_order() {
    let edges = vec![dep("a", "b"), dep("b", "c")];
    let sched = scheduler(8);
    let sp = ScriptSpawner::new(None);

    let dag1 = RoadmapDag {
        nodes: vec![node("a", "todo"), node("b", "todo"), node("c", "todo")],
        edges: edges.clone(),
    };
    assert_eq!(sched.plan(&dag1, 0).expect("tick 1").batch, vec!["a"]);
    assert_eq!(sched.dispatch(&dag1, 0, &sp).expect("tick 1").len(), 1);
    assert_eq!(sp.call_log(), vec!["a"]);

    let dag2 = RoadmapDag {
        nodes: vec![node("a", "done"), node("b", "todo"), node("c", "todo")],
        edges: edges.clone(),
    };
    assert_eq!(sched.plan(&dag2, 1).expect("tick 2").batch, vec!["b"]);
    assert_eq!(sched.dispatch(&dag2, 1, &sp).expect("tick 2").len(), 1);
    assert_eq!(sp.call_log(), vec!["a", "b"]);

    let dag3 = RoadmapDag {
        nodes: vec![node("a", "done"), node("b", "done"), node("c", "todo")],
        edges,
    };
    assert_eq!(sched.plan(&dag3, 1).expect("tick 3").batch, vec!["c"]);
    assert_eq!(sched.dispatch(&dag3, 1, &sp).expect("tick 3").len(), 1);
    assert_eq!(sp.call_log(), vec!["a", "b", "c"]);
}

#[test]
fn cap_one_serializes_without_deadlock() {
    let sched = scheduler(1);
    let sp = ScriptSpawner::new(None);
    let edges = vec![dep("a", "b"), dep("b", "c")];

    let dag1 = RoadmapDag {
        nodes: vec![node("a", "todo"), node("b", "todo"), node("c", "todo")],
        edges: edges.clone(),
    };
    let plan1 = sched.plan(&dag1, 0).expect("tick 1");
    assert_eq!(plan1.free_slots, 1);
    assert_eq!(plan1.batch, vec!["a"]);
    sched.dispatch(&dag1, 0, &sp).expect("tick 1 dispatch");

    let dag2 = RoadmapDag {
        nodes: vec![node("a", "done"), node("b", "todo"), node("c", "todo")],
        edges: edges.clone(),
    };
    assert_eq!(sched.plan(&dag2, 0).expect("tick 2").batch, vec!["b"]);
    sched.dispatch(&dag2, 0, &sp).expect("tick 2 dispatch");

    let dag3 = RoadmapDag {
        nodes: vec![node("a", "done"), node("b", "done"), node("c", "todo")],
        edges,
    };
    assert_eq!(sched.plan(&dag3, 0).expect("tick 3").batch, vec!["c"]);
    sched.dispatch(&dag3, 0, &sp).expect("tick 3 dispatch");

    assert_eq!(sp.call_log(), vec!["a", "b", "c"]);
}

#[test]
fn active_teammates_reduces_free_slots() {
    let sched = scheduler(4);
    let sp = ScriptSpawner::new(None);
    let dag = RoadmapDag {
        nodes: vec![
            node("a", "todo"),
            node("b", "todo"),
            node("c", "todo"),
            node("d", "todo"),
        ],
        edges: vec![],
    };

    let plan1 = sched.plan(&dag, 3).expect("tick 1");
    assert_eq!(plan1.free_slots, 1);
    assert_eq!(plan1.batch.len(), 1);
    assert_eq!(sched.dispatch(&dag, 3, &sp).expect("tick 1").len(), 1);

    let plan2 = sched.plan(&dag, 0).expect("tick 2");
    assert_eq!(plan2.free_slots, 4);
    assert_eq!(plan2.batch.len(), 4);
    assert_eq!(sched.dispatch(&dag, 0, &sp).expect("tick 2").len(), 4);

    assert_eq!(sp.call_log().len(), 5);
}
