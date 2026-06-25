//! E2 regression: a `DEPENDS_ON` cycle is detected and named, not silently
//! stalled. Covers the 2-node and 3-node cycles the card mandates, plus the
//! acyclic and dangling-dep negatives.
use super::{cycle_message, detect_cycle};
use kavach_surreal::MemoryEntry;

/// Minimal roadmap entry: a key + content carrying its `DEPENDS_ON:` line.
fn card(key: &str, deps: &[&str]) -> MemoryEntry {
    let content = if deps.is_empty() {
        String::new()
    } else {
        format!("DEPENDS_ON: {}", deps.join(", "))
    };
    MemoryEntry {
        id: None,
        project: surrealdb_types::RecordId::new("project", "t"),
        category: Some("roadmap".into()),
        entry_key: key.to_owned(),
        title: key.to_owned(),
        content,
        status: None,
        entry_status: Some("todo".into()),
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
fn two_node_cycle_is_detected_and_names_both_keys() {
    let cards = vec![card("a", &["b"]), card("b", &["a"])];
    let cycle = detect_cycle(&cards).expect("A->B->A is a cycle");
    assert!(
        cycle.contains(&"a".to_owned()) && cycle.contains(&"b".to_owned()),
        "both keys named: {cycle:?}"
    );
    let msg = cycle_message(&cycle);
    assert!(
        msg.starts_with("[DAG_CYCLE]"),
        "allow-stop marker present: {msg}"
    );
    assert!(
        msg.contains('a') && msg.contains('b'),
        "message names both keys: {msg}"
    );
}

#[test]
fn three_node_cycle_is_detected() {
    let cards = vec![card("x", &["y"]), card("y", &["z"]), card("z", &["x"])];
    let cycle = detect_cycle(&cards).expect("X->Y->Z->X is a cycle");
    for k in ["x", "y", "z"] {
        assert!(
            cycle.contains(&k.to_owned()),
            "key {k} on the 3-cycle: {cycle:?}"
        );
    }
}

#[test]
fn acyclic_dag_returns_none() {
    // a -> b -> c, plus an independent d. No back edge anywhere.
    let cards = vec![
        card("a", &["b"]),
        card("b", &["c"]),
        card("c", &[]),
        card("d", &[]),
    ];
    assert!(
        detect_cycle(&cards).is_none(),
        "a linear chain is not a cycle"
    );
}

#[test]
fn dangling_dep_is_not_a_cycle() {
    // a depends on a key that doesn't exist as a node — a missing dep, not a loop.
    let cards = vec![card("a", &["ghost-key"])];
    assert!(
        detect_cycle(&cards).is_none(),
        "a dep on an absent node is not a cycle"
    );
}

#[test]
fn self_loop_is_a_cycle() {
    // a depends on itself — the smallest cycle; must not stall silently.
    let cards = vec![card("a", &["a"])];
    let cycle = detect_cycle(&cards).expect("a self-dep is a 1-node cycle");
    assert_eq!(cycle, vec!["a".to_owned()]);
}
