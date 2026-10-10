//! Proves the DAG projection: tier assignment by dependency depth, READY vs
//! BLOCKED markers from prereq status, cycle is surfaced (not placed on a tier),
//! and the mermaid emit is a well-formed `flowchart TD`.
use super::{
    dag_from_roadmap, is_ready, render_mermaid, render_tiered_text, tiers,
};
use kavach_db::{DagEdge, DagNode, MemoryEntry, RoadmapDag};
use std::collections::HashMap;

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
        kavach_db::graph::roadmap_dag::TopoOrder::Ordered(o) => o,
        kavach_db::graph::roadmap_dag::TopoOrder::Cycle(_) => panic!("chain is acyclic"),
    };
    let depth = tiers(&order, &d.edges);
    assert_eq!(depth.get("p/roadmap/a"), Some(&0), "root is tier 0");
    assert_eq!(depth.get("p/roadmap/b"), Some(&1));
    assert_eq!(depth.get("p/roadmap/c"), Some(&2));
}

#[test]
fn tiered_text_marks_ready_and_blocked() {
    let out = render_tiered_text(&chain(), false);
    // 'a' has no prereqs -> READY; 'b' depends on unfinished 'a' -> WAITING.
    assert!(out.contains("TIER 0 — ready now"));
    assert!(out.contains("a — title a  ✓READY"), "got:\n{out}");
    assert!(
        out.contains("⏳WAITING"),
        "dependent must be waiting on its prereq:\n{out}"
    );
    assert!(out.contains("⤷ depends-on: a"), "inline prereq:\n{out}");
}

#[test]
fn ready_when_prereq_verified() {
    // a is verified -> b becomes READY despite the edge.
    let d = RoadmapDag {
        nodes: vec![node("a", "verified"), node("b", "todo")],
        edges: vec![dep("a", "b")],
    };
    let out = render_tiered_text(&d, false);
    assert!(
        out.contains("b — title b  ✓READY"),
        "verified prereq unblocks:\n{out}"
    );
}

#[test]
fn cycle_is_surfaced_not_tiered() {
    // a -> b -> a is a deadlock.
    let d = RoadmapDag {
        nodes: vec![node("a", "todo"), node("b", "todo")],
        edges: vec![dep("a", "b"), dep("b", "a")],
    };
    let out = render_tiered_text(&d, false);
    assert!(out.contains("⚠ CYCLE"), "cycle must be named:\n{out}");
    assert!(
        !out.contains("TIER"),
        "no tiers rendered for a cyclic graph:\n{out}"
    );
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

/// A minimal roadmap `MemoryEntry` (`todo`) carrying `content`.
fn row(key: &str, content: &str) -> MemoryEntry {
    MemoryEntry {
        id: None,
        project: "p".to_owned(),
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
        exec_prompt: None,
        occupied_by: None,
        occupied_until: None,
    }
}

#[test]
fn builds_dag_from_roadmap_rows_via_declared_deps() {
    use super::dag_from_roadmap;
    use kavach_db::MemoryEntry;
    let _ = std::marker::PhantomData::<MemoryEntry>;
    // u2 declares DEPENDS_ON: u1; u3 depends on u2 + an absent 'ghost' key.
    // An absent key is NOT dropped (that falsely marked the dependent ready —
    // Finding B): it becomes a phantom prereq node so the dependent shows BLOCKED,
    // matching the scheduler which resolves deps against the GLOBAL pool.
    let rows = vec![
        row("u1", "no deps"),
        row("u2", "DEPENDS_ON: u1"),
        row("u3", "DEPENDS_ON: u2 ghost"),
    ];
    let dag = dag_from_roadmap(&rows);
    assert_eq!(
        dag.nodes.len(),
        4,
        "3 rows + 1 phantom node for absent 'ghost'"
    );
    assert_eq!(
        dag.edges.len(),
        3,
        "u1->u2, u2->u3, ghost->u3 (phantom kept)"
    );
    assert!(
        dag.edges
            .iter()
            .any(|e| e.source == "u1" && e.target == "u2")
    );
    assert!(
        dag.edges
            .iter()
            .any(|e| e.source == "u2" && e.target == "u3")
    );
    assert!(
        dag.edges
            .iter()
            .any(|e| e.source == "ghost" && e.target == "u3"),
        "absent dep surfaced as a phantom prereq, not silently dropped"
    );
    let ghost = dag
        .nodes
        .iter()
        .find(|n| n.entry_key == "ghost")
        .expect("phantom present");
    assert_eq!(
        ghost.entry_status, "missing",
        "phantom never verified/done -> dependent stays blocked"
    );
}

/// REGRESSION (`r-kavach-depends-on-ghost-nodes`): the daemon sink mirrors
/// graph-edge targets into the `DEPENDS_ON:` line PROJECT-QUALIFIED
/// (`<project>/<category>/<key>`), while node ids are bare `entry_key`s.
/// Without `bare_dep_key` normalization every real dep ALSO matched nothing,
/// so each `--depends-on` edge produced a bogus "(unresolved)" phantom node —
/// which is why `kavach db kanban` reported 230 nodes for ~120 real cards.
#[test]
fn project_qualified_dep_does_not_create_a_phantom_node() {
    let rows = vec![
        row("roadmap.stage1-backend", "no deps"),
        row(
            "roadmap.fusion-canon",
            "DEPENDS_ON: vectron/roadmap/roadmap.stage1-backend",
        ),
    ];
    let dag = dag_from_roadmap(&rows);
    assert_eq!(
        dag.nodes.len(),
        2,
        "a real prerequisite in qualified form must NOT add a phantom node"
    );
    assert!(
        !dag.nodes
            .iter()
            .any(|n| n.title.contains("unresolved")),
        "no phantom '(unresolved)' node for a dep that IS present:\n{:?}",
        dag.nodes
    );
    assert!(
        dag.edges
            .iter()
            .any(|e| e.source == "roadmap.stage1-backend"
                && e.target == "roadmap.fusion-canon"),
        "the edge is real and points at the bare key:\n{:?}",
        dag.edges
    );
    // And the dependent must be BLOCKED, not falsely READY, while its prereq is open.
    let by_id: HashMap<&str, &DagNode> =
        dag.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let dependent = by_id.get("roadmap.fusion-canon").expect("dependent");
    assert!(
        !is_ready(dependent, &dag.edges, &by_id),
        "prereq is still todo, so the dependent is not ready"
    );
}

/// A qualified dep that genuinely does not exist anywhere still yields exactly
/// ONE phantom — normalization must not collapse or duplicate it.
#[test]
fn qualified_but_absent_dep_still_yields_one_phantom() {
    let rows = vec![row(
        "roadmap.child",
        "DEPENDS_ON: other-project/roadmap/roadmap.truly-absent",
    )];
    let dag = dag_from_roadmap(&rows);
    assert_eq!(dag.nodes.len(), 2, "1 card + 1 phantom");
    let phantom = dag
        .nodes
        .iter()
        .find(|n| n.entry_status == "missing")
        .expect("phantom");
    assert_eq!(
        phantom.entry_key, "roadmap.truly-absent",
        "phantom is keyed by the BARE tail so it is nameable and matchable"
    );
}

/// Build a `MTICKET v1` body declaring `conflicts_with`.
fn ticket_body(conflicts: &[&str]) -> String {
    let mut t = kavach_db::microticket::MicroTicket {
        kind: kavach_db::microticket::TicketKind::Chore,
        title: "A micro ticket that stands alone".to_owned(),
        goal: "The conflict relation is honoured at dispatch.".to_owned(),
        modality: kavach_db::microticket::Modality::Must,
        change: kavach_db::microticket::Change {
            op: kavach_db::microticket::Op::Edit,
            when: "a sibling card declares a conflict".to_owned(),
            then: "neither card reports READY".to_owned(),
        },
        scope: vec!["crates/kavach-cli/src/cmd/db/kanban/dag_render.rs".to_owned()],
        do_not_touch: vec!["crates/kavach-cli/src/cli/db.rs".to_owned()],
        verify: vec![kavach_db::microticket::VerifyStep {
            method: kavach_db::microticket::VerifyMethod::Test,
            run: Some("cargo test -p kavach-cli dag_render".to_owned()),
            expect: "neither conflicting card is marked ready".to_owned(),
        }],
        source: None,
        depends_on: vec![],
        conflicts_with: conflicts.iter().map(|s| (*s).to_owned()).collect(),
        supersedes: vec![],
        manual_by: None,
    };
    // `do_not_touch` must not overlap `scope` or validation fails; adjust the
    // sample so the body is genuinely valid, not merely parseable.
    t.do_not_touch = vec!["crates/kavach-cli/src/cli/db.rs".to_owned()];
    format!("MTICKET v1\n{}", t.to_json().expect("serialize ticket"))
}

/// REGRESSION: mutual exclusion is ENFORCED, not merely displayed. Two open
/// cards declaring `conflicts_with` must never both be dispatchable, whatever
/// their dependencies say — otherwise a parallel runner selects both sides.
#[test]
fn conflicting_cards_are_never_both_ready() {
    let rows = vec![
        row("mt.a.one", &ticket_body(&["mt.a.two"])),
        row("mt.a.two", "no deps"),
    ];
    let dag = dag_from_roadmap(&rows);
    let by_id: HashMap<&str, &DagNode> =
        dag.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    assert!(
        dag.edges.iter().any(|e| e.rel == "conflicts_with"),
        "the conflict edge must reach the DAG:\n{:?}",
        dag.edges
    );
    for id in ["mt.a.one", "mt.a.two"] {
        let n = by_id.get(id).expect("node");
        assert!(
            !is_ready(n, &dag.edges, &by_id),
            "{id} conflicts with an OPEN card and must not be dispatchable"
        );
    }
}

/// Closing one side resolves the conflict for the other — otherwise a conflict
/// would be permanent and the pair could never be worked.
#[test]
fn closing_one_side_unblocks_the_other() {
    let mut rows = vec![row("mt.a.one", &ticket_body(&["mt.a.two"]))];
    let mut closed = row("mt.a.two", "no deps");
    closed.entry_status = Some("done".to_owned());
    rows.push(closed);
    let dag = dag_from_roadmap(&rows);
    let by_id: HashMap<&str, &DagNode> =
        dag.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let survivor = by_id.get("mt.a.one").expect("node");
    assert!(
        is_ready(survivor, &dag.edges, &by_id),
        "the remaining open card is unblocked once its rival is done"
    );
}

/// A non-ticket body must not break the board: unknown shapes contribute no
/// conflict edges rather than failing the whole render.
#[test]
fn non_ticket_body_contributes_no_conflicts() {
    let rows = vec![
        row("mt.a.one", "DEPENDS_ON: mt.a.two"),
        row("mt.a.two", "plain prose, not a ticket"),
    ];
    let dag = dag_from_roadmap(&rows);
    assert!(
        !dag.edges.iter().any(|e| e.rel == "conflicts_with"),
        "no ticket body means no conflict edges"
    );
    let by_id: HashMap<&str, &DagNode> =
        dag.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let two = by_id.get("mt.a.two").expect("node");
    assert!(is_ready(two, &dag.edges, &by_id), "dep satisfied, no conflict");
}

#[test]
fn empty_dag_is_safe() {
    let out = render_tiered_text(&RoadmapDag::default(), false);
    assert!(
        out.contains("0 node(s), 0 edge(s)"),
        "empty boundary:\n{out}"
    );
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
    let out = render_tiered_text(&d, false);
    // 'a' is todo with no deps -> READY in TIER 0.
    assert!(
        out.contains("a — title a  ✓READY"),
        "open todo is ready:\n{out}"
    );
    // 'b' and 'c' are closed -> should NOT appear in the tier output at all.
    // They have no dependencies, so they would appear in TIER 0 if the bug existed.
    assert!(
        !out.contains("b — title b"),
        "verified card must NOT appear in tiers:\n{out}"
    );
    assert!(
        !out.contains("c — title c"),
        "done card must NOT appear in tiers:\n{out}"
    );
}

#[test]
fn show_closed_keeps_done_tier_for_explicit_filter() {
    // K-5: `--status done` asks for closed rows — they must render as tiers
    // instead of being hidden as "already closed".
    let d = RoadmapDag {
        nodes: vec![node("a", "todo"), node("c", "done")],
        edges: vec![],
    };
    let hidden = render_tiered_text(&d, false);
    assert!(
        !hidden.contains("c — title c"),
        "default view hides done:\n{hidden}"
    );
    let shown = render_tiered_text(&d, true);
    assert!(
        shown.contains("c — title c"),
        "explicit done filter shows done:\n{shown}"
    );
}
