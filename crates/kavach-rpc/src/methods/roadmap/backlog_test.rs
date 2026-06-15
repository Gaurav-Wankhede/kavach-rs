use crate::methods::roadmap::readiness::{deps_satisfied, is_runnable_status};

#[expect(
    dead_code,
    reason = "shared test fixture builder; not every test in this module exercises every helper"
)]
fn entry(key: &str, status: &str, content: &str) -> kavach_surreal::MemoryEntry {
    kavach_surreal::MemoryEntry {
        id: None,
        project: surrealdb_types::RecordId::new("project", "t"),
        category: Some("roadmap".into()),
        entry_key: key.into(),
        title: key.into(),
        content: content.into(),
        status: None,
        entry_status: Some(status.into()),
        tags: None,
        decay_score: None,
        access_count: None,
        created_at: None,
        updated_at: None,
        priority: None,
        lane: None,
    }
}

fn entry_with_priority(
    key: &str,
    status: &str,
    priority: Option<i64>,
) -> kavach_surreal::MemoryEntry {
    kavach_surreal::MemoryEntry {
        id: None,
        project: surrealdb_types::RecordId::new("project", "t"),
        category: Some("roadmap".into()),
        entry_key: key.into(),
        title: key.into(),
        content: String::new(),
        status: None,
        entry_status: Some(status.into()),
        tags: None,
        decay_score: None,
        access_count: None,
        created_at: None,
        updated_at: None,
        priority,
        lane: None,
    }
}

fn pick_first(entries: &[kavach_surreal::MemoryEntry]) -> Option<&kavach_surreal::MemoryEntry> {
    entries
        .iter()
        .find(|e| is_runnable_status(e.entry_status_str()) && deps_satisfied(e, entries))
}

#[test]
fn priority_one_wins_over_priority_two_and_null() {
    let entries = vec![
        entry_with_priority("p1.urgent", "todo", Some(1)),
        entry_with_priority("p2.normal", "todo", Some(2)),
        entry_with_priority("p_null.lowest", "todo", None),
    ];
    let picked = pick_first(&entries).expect("runnable exists");
    assert_eq!(picked.entry_key, "p1.urgent");
}

#[test]
fn null_priorities_break_tie_by_creation_order_within_null_bucket() {
    let entries = vec![
        entry_with_priority("older.null", "todo", None),
        entry_with_priority("newer.null", "todo", None),
    ];
    let picked = pick_first(&entries).expect("runnable exists");
    assert_eq!(picked.entry_key, "older.null");
}

#[test]
fn priority_outranks_creation_order() {
    let entries = vec![
        entry_with_priority("p1.todo", "todo", Some(1)),
        entry_with_priority("legacy.in_progress", "in_progress", None),
    ];
    let picked = pick_first(&entries).expect("runnable exists");
    assert_eq!(picked.entry_key, "p1.todo");
}

#[test]
fn priority_skipped_when_deps_unmet() {
    let mut entries = vec![
        entry_with_priority("p1.gated", "todo", Some(1)),
        entry_with_priority("p3.ready", "todo", Some(3)),
    ];
    if let Some(first) = entries.first_mut() {
        first.content = "BLOCKED_BY: missing.dep".into();
    }
    let picked = pick_first(&entries).expect("runnable exists");
    assert_eq!(picked.entry_key, "p3.ready");
}

#[test]
fn non_canonical_status_string_is_skipped_for_runnable_todo() {
    let entries = vec![
        entry_with_priority("b.legacy.row", "legacy", Some(1)),
        entry_with_priority("c.todo.normal", "todo", Some(2)),
    ];
    let picked = pick_first(&entries).expect("runnable exists");
    assert_eq!(picked.entry_key, "c.todo.normal");
}

#[test]
fn promote_next_backlog_selects_runnable_todo_head() {
    assert!(
        !is_runnable_status("legacy"),
        "a non-canonical status must never be runnable"
    );
    let entries = vec![
        entry_with_priority("a.legacy.row", "legacy", Some(1)),
        entry_with_priority("b.backlog.todo", "todo", Some(2)),
    ];
    let picked = pick_first(&entries).expect("a runnable todo head must be found");
    assert_eq!(picked.entry_key, "b.backlog.todo");
}
