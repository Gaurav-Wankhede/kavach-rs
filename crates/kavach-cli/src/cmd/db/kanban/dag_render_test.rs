//! Proves the DAG projection: tier assignment by dependency depth, READY vs
//! BLOCKED markers from prereq status, cycle is surfaced (not placed on a tier),
//! and the mermaid emit is a well-formed `flowchart TD`.
use super::{render_mermaid, render_tiered_text, tiers};
use kavach_surreal::{DagEdge, DagNode, RoadmapDag};

fn node(key: &str, status: &str) -> DagNode {
    DagNode {
        id: format!("p/roadmap/{key}"),
        entry_key: key.to_owned(),
        title: format!("title {key}"),
        entry_status: status.to_owned(),
        category: "roadmap".to_owned(),
    }
}

fn dep(from: &str, to: &str) -> DagEdge {
    DagEdge {
        source: format!("p/roadmap/{from}"),
        target: format!("p/roadmap/{to}"),
        rel: "depends_on".to_owned(),
    }
}

/// a -> b -> c chain: tiers must be 0, 1, 2.
fn chain() -> RoadmapDag {
    RoadmapDag {
        nodes: vec![node("a", "todo"), node("b", "todo"), node("c", "todo")],
        edges: vec![dep("a", "b"), dep("b", "c")],
    }
}

#[test]
fn tiers_follow_dependency_depth() {
    let d = chain();
    let order = match d.toposort_or_cycle() {
        kavach_surreal::graph::roadmap_dag::TopoOrder::Ordered(o) => o,
        kavach_surreal::graph::roadmap_dag::TopoOrder::Cycle(_) => panic!("chain is acyclic"),
    };
    let depth = tiers(&order, &d.edges);
    assert_eq!(depth.get("p/roadmap/a"), Some(&0), "root is tier 0");
    assert_eq!(depth.get("p/roadmap/b"), Some(&1));
    assert_eq!(depth.get("p/roadmap/c"), Some(&2));
}

#[test]
fn tiered_text_marks_ready_and_blocked() {
    let out = render_tiered_text(&chain());
    // 'a' has no prereqs -> READY; 'b' depends on unfinished 'a' -> WAITING.
    assert!(out.contains("TIER 0 — ready now"));
    assert!(out.contains("a — title a  ✓READY"), "got:\n{out}");
    assert!(out.contains("⏳WAITING"), "dependent must be waiting on its prereq:\n{out}");
    assert!(out.contains("⤷ depends-on: a"), "inline prereq:\n{out}");
}

#[test]
fn ready_when_prereq_verified() {
    // a is verified -> b becomes READY despite the edge.
    let d = RoadmapDag {
        nodes: vec![node("a", "verified"), node("b", "todo")],
        edges: vec![dep("a", "b")],
    };
    let out = render_tiered_text(&d);
    assert!(out.contains("b — title b  ✓READY"), "verified prereq unblocks:\n{out}");
}

#[test]
fn cycle_is_surfaced_not_tiered() {
    // a -> b -> a is a deadlock.
    let d = RoadmapDag {
        nodes: vec![node("a", "todo"), node("b", "todo")],
        edges: vec![dep("a", "b"), dep("b", "a")],
    };
    let out = render_tiered_text(&d);
    assert!(out.contains("⚠ CYCLE"), "cycle must be named:\n{out}");
    assert!(!out.contains("TIER"), "no tiers rendered for a cyclic graph:\n{out}");
}

#[test]
fn mermaid_emits_flowchart_with_edges() {
    let m = render_mermaid(&chain());
    assert!(m.starts_with("flowchart TD"));
    // sanitized ids (slashes -> underscores) + an arrow per dep edge.
    assert!(m.contains("p_roadmap_a"), "node id sanitized:\n{m}");
    assert!(m.contains("-->"), "edges rendered as arrows:\n{m}");
    assert_eq!(m.matches("-->").count(), 2, "two dep edges:\n{m}");
}

#[test]
fn builds_dag_from_roadmap_rows_via_declared_deps() {
    use super::dag_from_roadmap;
    use kavach_surreal::MemoryEntry;
    let row = |key: &str, content: &str| MemoryEntry {
        id: None,
        project: surrealdb_types::RecordId::new("project", "p"),
        category: Some("roadmap".to_owned()),
        entry_key: key.to_owned(),
        title: format!("title {key}"),
        content: content.to_owned(),
        status: None,
        entry_status: Some("todo".to_owned()),
        tags: None,
        decay_score: None,
        access_count: None,
        created_at: None,
        updated_at: None,
        priority: None,
        lane: None,
        occupied_by: None,
        occupied_until: None,
    };
    // u2 declares DEPENDS_ON: u1; u3 depends on u2 + an absent 'ghost' key.
    // An absent key is NOT dropped (that falsely marked the dependent ready —
    // Finding B): it becomes a phantom prereq node so the dependent shows BLOCKED,
    // matching the scheduler which resolves deps against the GLOBAL pool.
    let rows = vec![
        row("u1", "no deps"),
        row("u2", "DEPENDS_ON: u1"),
        row("u3", "BLOCKED_BY: u2 ghost"),
    ];
    let dag = dag_from_roadmap(&rows);
    assert_eq!(dag.nodes.len(), 4, "3 rows + 1 phantom node for absent 'ghost'");
    assert_eq!(dag.edges.len(), 3, "u1->u2, u2->u3, ghost->u3 (phantom kept)");
    assert!(dag.edges.iter().any(|e| e.source == "u1" && e.target == "u2"));
    assert!(dag.edges.iter().any(|e| e.source == "u2" && e.target == "u3"));
    assert!(
        dag.edges.iter().any(|e| e.source == "ghost" && e.target == "u3"),
        "absent dep surfaced as a phantom prereq, not silently dropped"
    );
    let ghost = dag.nodes.iter().find(|n| n.entry_key == "ghost").expect("phantom present");
    assert_eq!(ghost.entry_status, "missing", "phantom never verified/done -> dependent stays blocked");
}

#[test]
fn empty_dag_is_safe() {
    let out = render_tiered_text(&RoadmapDag::default());
    assert!(out.contains("0 node(s), 0 edge(s)"), "empty boundary:\n{out}");
}

#[test]
fn closed_cards_not_in_ready_now() {
    // Verify that verified/done cards do NOT appear in TIER 0 "ready now",
    // even if all their prerequisites are satisfied.
    // Bug: is_ready() checked prereqs only, ignoring node.entry_status.
    let d = RoadmapDag {
        nodes: vec![
            node("a", "todo"),
            node("b", "verified"), // closed: should NOT be in ready now
            node("c", "done"),     // closed: should NOT be in ready now
        ],
        edges: vec![],
    };
    let out = render_tiered_text(&d);
    // 'a' is todo with no deps -> READY in TIER 0.
    assert!(out.contains("a — title a  ✓READY"), "open todo is ready:\n{out}");
    // 'b' and 'c' are closed -> should NOT appear in the tier output at all.
    // They have no dependencies, so they would appear in TIER 0 if the bug existed.
    assert!(!out.contains("b — title b"), "verified card must NOT appear in tiers:\n{out}");
    assert!(!out.contains("c — title c"), "done card must NOT appear in tiers:\n{out}");
}
