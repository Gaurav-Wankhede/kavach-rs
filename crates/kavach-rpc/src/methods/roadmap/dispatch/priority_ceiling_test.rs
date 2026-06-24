//! E3 regression: a low-urgency blocker inherits its urgent dependent's priority
//! so the urgent card isn't starved. Pure test of the inversion + sort key.
use super::{effective_priorities, sort_by_effective_priority};
use kavach_surreal::MemoryEntry;

/// Entry with a priority + optional `DEPENDS_ON` line.
fn card(key: &str, priority: Option<i64>, deps: &[&str]) -> MemoryEntry {
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
        priority,
        lane: None,
        occupied_by: None,
        occupied_until: None,
    }
}

#[test]
fn blocker_inherits_urgent_dependents_priority() {
    // urgent A (pri 1) DEPENDS_ON low-urgency B (pri 9). B must rise to 1.
    let cards = vec![card("a", Some(1), &["b"]), card("b", Some(9), &[])];
    let eff = effective_priorities(&cards);
    assert_eq!(
        eff.get("b"),
        Some(&1),
        "blocker B inherits dependent A's urgency"
    );
    assert_eq!(eff.get("a"), Some(&1), "A keeps its own priority");
}

#[test]
fn ceiling_takes_the_most_urgent_of_many_dependents() {
    // B blocks both A (pri 5) and C (pri 2). B inherits the MOST urgent = 2.
    let cards = vec![
        card("a", Some(5), &["b"]),
        card("c", Some(2), &["b"]),
        card("b", Some(9), &[]),
    ];
    let eff = effective_priorities(&cards);
    assert_eq!(
        eff.get("b"),
        Some(&2),
        "B inherits min(5,2)=2, the hungriest dependent"
    );
}

#[test]
fn no_dependents_keeps_raw_priority() {
    let cards = vec![card("solo", Some(7), &[])];
    assert_eq!(effective_priorities(&cards).get("solo"), Some(&7));
}

#[test]
fn sort_dispatches_the_lifted_blocker_first() {
    // Board: B (pri 9) blocks urgent A (pri 1); an unrelated D (pri 2).
    // WITHOUT ceiling: order by raw pri is A(1), D(2), B(9) — but A is blocked by
    // B, so dispatch would run D then stall on A behind B. WITH ceiling B rises
    // to 1, so B sorts to the FRONT and dispatches first, unblocking A.
    let mut cards = vec![
        card("a", Some(1), &["b"]),
        card("d", Some(2), &[]),
        card("b", Some(9), &[]),
    ];
    sort_by_effective_priority(&mut cards);
    let order: Vec<&str> = cards.iter().map(|c| c.entry_key.as_str()).collect();
    let pos = |k: &str| order.iter().position(|x| *x == k).expect("present");
    // The inversion fix: B (lifted to 1) must sort AHEAD of the unrelated D
    // (pri 2). A ties B at pri-1 and may precede it (A is filtered out by
    // deps_satisfied at pick time since B isn't done), but B MUST beat D — that
    // is what unblocks A. Before the ceiling, B (raw 9) sorted behind D.
    assert!(
        pos("b") < pos("d"),
        "lifted blocker B must outrank unrelated D: {order:?}"
    );
}

#[test]
fn absent_priority_sorts_last() {
    let mut cards = vec![card("none", None, &[]), card("p1", Some(1), &[])];
    sort_by_effective_priority(&mut cards);
    assert_eq!(cards.first().map(|c| c.entry_key.as_str()), Some("p1"));
    assert_eq!(cards.last().map(|c| c.entry_key.as_str()), Some("none"));
}
