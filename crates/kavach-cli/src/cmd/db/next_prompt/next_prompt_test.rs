use super::{Pick, pick};
use kavach_surreal::MemoryEntry;

fn card(key: &str, status: &str, exec_prompt: Option<&str>) -> MemoryEntry {
    MemoryEntry {
        id: None,
        project: surrealdb_types::RecordId::new("project", "t"),
        category: Some("roadmap".into()),
        entry_key: key.to_owned(),
        title: "t".to_owned(),
        content: String::new(),
        status: None,
        entry_status: Some(status.to_owned()),
        tags: None,
        decay_score: None,
        access_count: None,
        created_at: None,
        updated_at: None,
        priority: None,
        lane: None,
        exec_prompt: exec_prompt.map(str::to_owned),
        occupied_by: None,
        occupied_until: None,
    }
}

#[test]
fn serves_first_todo_with_a_prompt() {
    let rows = [
        card("done-one", "done", Some("ignored")),
        card("top", "todo", Some("run X precisely")),
        card("later", "todo", Some("other")),
    ];
    match pick(&rows) {
        Pick::Prompt(p) => assert_eq!(p, "run X precisely"),
        _ => panic!("expected the first todo card's prompt"),
    }
}

#[test]
fn top_todo_without_prompt_is_missing_not_skipped() {
    let rows = [card("top", "todo", None), card("later", "todo", Some("x"))];
    match pick(&rows) {
        Pick::Missing(key) => assert_eq!(key, "top"),
        _ => panic!("a promptless top todo must surface as Missing, never skipped"),
    }
}

#[test]
fn whitespace_only_prompt_counts_as_missing() {
    let rows = [card("top", "todo", Some("   \n  "))];
    assert!(matches!(pick(&rows), Pick::Missing(_)));
}

#[test]
fn no_todo_card_is_empty() {
    let rows = [card("d", "done", Some("x")), card("v", "verified", None)];
    assert!(matches!(pick(&rows), Pick::Empty));
}
