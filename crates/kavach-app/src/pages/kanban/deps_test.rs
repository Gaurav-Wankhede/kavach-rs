//! Proves the GUI dependency-awareness mirror: a card with an unsatisfied
//! declared dep is BLOCKED; one whose deps are all done/verified (or has none,
//! or whose dep key is absent) is not.
use super::{is_blocked, status_index};
use crate::state::EntryRef;
use kavach_types::MemoryStatus;

fn card(key: &str, status: MemoryStatus, content: &str) -> EntryRef {
    EntryRef {
        project_slug: "p".to_owned(),
        category: "roadmap".to_owned(),
        key: key.to_owned(),
        title: format!("t {key}"),
        content: content.to_owned(),
        status,
    }
}

#[test]
fn blocked_when_prereq_open() {
    let rows = vec![
        card("a", MemoryStatus::Todo, ""),
        card("b", MemoryStatus::Todo, "DEPENDS_ON: a"),
    ];
    let idx = status_index(&rows);
    assert!(!is_blocked(&rows[0], &idx), "a has no deps -> ready");
    assert!(is_blocked(&rows[1], &idx), "b depends on open a -> blocked");
}

#[test]
fn unblocked_when_prereq_done_or_verified() {
    let rows = vec![
        card("a", MemoryStatus::Verified, ""),
        card("b", MemoryStatus::Done, ""),
        card("c", MemoryStatus::Todo, "DEPENDS_ON: a, b"),
    ];
    let idx = status_index(&rows);
    assert!(!is_blocked(&rows[2], &idx), "all prereqs done/verified -> ready");
}

#[test]
fn dangling_dep_does_not_block() {
    let rows = vec![card("c", MemoryStatus::Todo, "BLOCKED_BY: ghost")];
    let idx = status_index(&rows);
    assert!(!is_blocked(&rows[0], &idx), "absent dep key cannot block");
}

#[test]
fn no_deps_never_blocked() {
    let rows = vec![card("a", MemoryStatus::Todo, "just a description, no deps")];
    let idx = status_index(&rows);
    assert!(!is_blocked(&rows[0], &idx));
}
