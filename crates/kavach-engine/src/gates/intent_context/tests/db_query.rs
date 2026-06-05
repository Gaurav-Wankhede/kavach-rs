//! `append_db_query_required` status-prompt + session-isolation tests.
use crate::gates::intent_context::db_query::append_db_query_required;

#[test]
fn test_db_query_required_progress() {
    let mut ctx = String::new();
    append_db_query_required(
        &mut ctx,
        "Now provide me the Progress of the project since Phase 1",
    );
    assert!(ctx.contains("DB_QUERY_REQUIRED"));
    assert!(ctx.contains("kavach db kanban"));
}

#[test]
fn test_db_query_required_status() {
    let mut ctx = String::new();
    append_db_query_required(&mut ctx, "What is the current status?");
    assert!(ctx.contains("DB_QUERY_REQUIRED"));
}

#[test]
fn test_db_query_required_resume() {
    let mut ctx = String::new();
    append_db_query_required(&mut ctx, "Resume the migration task");
    assert!(ctx.contains("DB_QUERY_REQUIRED"));
}

#[test]
fn test_db_query_not_required_for_code() {
    let mut ctx = String::new();
    append_db_query_required(&mut ctx, "Fix the compile error in auth.rs");
    assert!(ctx.is_empty());
}

#[test]
fn test_db_query_required_next_task() {
    let mut ctx = String::new();
    append_db_query_required(&mut ctx, "Provide me the Next task immediately");
    assert!(ctx.contains("DB_QUERY_REQUIRED"));
    assert!(ctx.contains("SESSION_PROJECT_ISOLATION"));
}

#[test]
fn test_db_query_required_whats_next() {
    let mut ctx = String::new();
    append_db_query_required(&mut ctx, "What's next?");
    assert!(ctx.contains("DB_QUERY_REQUIRED"));
}
