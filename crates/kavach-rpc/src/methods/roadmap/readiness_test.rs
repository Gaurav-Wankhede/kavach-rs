use crate::methods::roadmap::readiness::{
    dep_key_satisfied, deps_satisfied, is_runnable_status, parse_declared_deps,
};

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
        occupied_by: None,
        occupied_until: None,
    }
}

#[test]
fn runnable_is_strictly_todo_or_in_progress() {
    assert!(is_runnable_status("todo"));
    assert!(is_runnable_status("in_progress"));
}

#[test]
fn legacy_and_terminal_statuses_are_not_runnable() {
    for s in ["done", "verified", "unknown", "legacy-garbage"] {
        assert!(!is_runnable_status(s), "{s} must not be runnable");
    }
}

#[test]
fn parse_deps_inline_and_bullet_forms() {
    assert!(parse_declared_deps("no deps here").is_empty());
    assert_eq!(
        parse_declared_deps("BLOCKED_BY: erp.phase1"),
        ["erp.phase1"]
    );
    assert_eq!(
        parse_declared_deps("DEPENDS_ON: a.one, b.two"),
        ["a.one", "b.two"]
    );
    let body = "GOAL: x\nDEPENDS_ON:\n  - dash.lld (Widget tree first)\n  - arch.mesh\nNEXT: y";
    assert_eq!(parse_declared_deps(body), ["dash.lld", "arch.mesh"]);
}

#[test]
fn parse_deps_prose_false_positive_is_fail_closed() {
    assert!(
        parse_declared_deps("As noted, BLOCKED_BY: must not appear inline.").is_empty(),
        "mid-line prose mention is NOT a declaration"
    );
    let literal_line = "BLOCKED_BY: looks-like-a-dep";
    assert_eq!(parse_declared_deps(literal_line), ["looks-like-a-dep"]);
    let all: Vec<kavach_surreal::MemoryEntry> = Vec::new();
    assert!(
        !dep_key_satisfied("looks-like-a-dep", &all),
        "an unresolvable parsed key is fail-closed"
    );
}

#[test]
fn dep_key_satisfied_only_for_verified_or_done() {
    let all = vec![
        entry("d.verified", "verified", ""),
        entry("d.done", "done", ""),
        entry("d.todo", "todo", ""),
    ];
    assert!(dep_key_satisfied("d.verified", &all));
    assert!(dep_key_satisfied("d.done", &all));
    assert!(!dep_key_satisfied("d.todo", &all));
    assert!(!dep_key_satisfied("d.missing", &all));
}

fn first(v: &[kavach_surreal::MemoryEntry]) -> &kavach_surreal::MemoryEntry {
    v.first().expect("fixture must be non-empty")
}

#[test]
fn leaf_card_with_no_deps_is_ready() {
    let all = vec![entry("leaf", "todo", "GOAL: just do it")];
    assert!(deps_satisfied(first(&all), &all));
}

#[test]
fn card_with_unmet_dep_is_not_ready() {
    let all = vec![
        entry("child", "todo", "BLOCKED_BY: parent"),
        entry("parent", "todo", ""),
    ];
    assert!(!deps_satisfied(first(&all), &all));
}

#[test]
fn card_with_met_dep_is_ready() {
    let all = vec![
        entry("child", "todo", "DEPENDS_ON: parent"),
        entry("parent", "verified", ""),
    ];
    assert!(deps_satisfied(first(&all), &all));
}

#[test]
fn loop_termination_all_todo_but_dep_blocked_yields_no_dispatchable() {
    let all = vec![
        entry("root", "todo", ""),
        entry("a", "todo", "BLOCKED_BY: root"),
        entry("b", "todo", "DEPENDS_ON: root"),
        entry("c", "todo", "BLOCKED_BY: missing.unbuilt.key"),
    ];
    let any_dispatchable = all
        .iter()
        .skip(1)
        .any(|e| is_runnable_status(e.entry_status_str()) && deps_satisfied(e, &all));
    assert!(
        !any_dispatchable,
        "dependents with unmet deps must not be dispatchable"
    );
}

#[test]
fn chain_unblocks_when_root_completes() {
    let all = vec![
        entry("root", "verified", ""),
        entry("a", "todo", "BLOCKED_BY: root"),
    ];
    let dependent = all.get(1).expect("fixture has 2 entries");
    assert!(
        deps_satisfied(dependent, &all),
        "once the root is verified, the dependent must become dispatchable"
    );
}

#[test]
fn done_and_verified_are_not_runnable() {
    assert!(!is_runnable_status("done"));
    assert!(!is_runnable_status("verified"));
}

#[test]
fn unknown_status_is_not_runnable() {
    assert!(!is_runnable_status(""));
    assert!(!is_runnable_status("garbage"));
    assert!(!is_runnable_status("legacy-status"));
}

#[test]
fn genuine_leaf_card_stays_dispatchable() {
    // No operator-gate path exists any more: a card with a runnable status and no
    // unmet dependency is dispatchable, full stop (operator directive 2026-06-16).
    let body = "Reusable Dioxus virtualization organism in ui-organisms. \
                Windowing + DOM node recycling + overscan buffer.";
    let e = entry("frontend.virtual-list-organism", "todo", body);
    assert!(
        is_runnable_status(e.entry_status_str()) && deps_satisfied(&e, std::slice::from_ref(&e)),
        "a genuine actionable leaf must be dispatchable"
    );
}

#[test]
fn dispatcher_picks_first_runnable_dependency_satisfied_leaf() {
    fn entry_with_priority(
        key: &str,
        status: &str,
        priority: Option<i64>,
        content: &str,
    ) -> kavach_surreal::MemoryEntry {
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
            priority,
            lane: None,
        occupied_by: None,
        occupied_until: None,
        }
    }
    fn pick_first(entries: &[kavach_surreal::MemoryEntry]) -> Option<&kavach_surreal::MemoryEntry> {
        entries
            .iter()
            .find(|e| is_runnable_status(e.entry_status_str()) && deps_satisfied(e, entries))
    }
    // The head declares an unmet dependency (its prerequisite is still todo), so
    // the dispatcher skips it on dependency order and picks the next ready leaf.
    let entries = vec![
        entry_with_priority("p1.head", "todo", Some(1), "DEPENDS_ON: p9.never-done"),
        entry_with_priority("p2.leaf", "todo", Some(2), "real agent work"),
    ];
    let picked = pick_first(&entries).expect("an actionable card must be found");
    assert_eq!(picked.entry_key, "p2.leaf");
}
