//! `append_db_query_required_with` status-prompt + session-isolation tests.
//! Tests call the PURE core (explicit `memory_queried` + `slug`) so the verdict
//! is fully determined by arguments — never by whatever on-disk session the test
//! process inherited. The thin `append_db_query_required` wrapper that reads the
//! global session is exercised by the live intent gate, not here.
use crate::gates::intent_context::db_query::append_db_query_required_with;

/// Status-shaped prompt with a fresh (un-queried) session -> block MUST fire.
fn fresh(ctx: &mut String, prompt: &str) {
    append_db_query_required_with(ctx, prompt, false, "kavach-rs");
}

#[test]
fn test_db_query_required_progress() {
    let mut ctx = String::new();
    fresh(&mut ctx, "Now provide me the Progress of the project since Phase 1");
    assert!(ctx.contains("DB_QUERY_REQUIRED"));
    assert!(ctx.contains("kavach db kanban"));
}

#[test]
fn test_db_query_required_status() {
    let mut ctx = String::new();
    fresh(&mut ctx, "What is the current status?");
    assert!(ctx.contains("DB_QUERY_REQUIRED"));
}

#[test]
fn test_db_query_required_resume() {
    let mut ctx = String::new();
    fresh(&mut ctx, "Resume the migration task");
    assert!(ctx.contains("DB_QUERY_REQUIRED"));
}

#[test]
fn test_db_query_not_required_for_code() {
    let mut ctx = String::new();
    fresh(&mut ctx, "Fix the compile error in auth.rs");
    assert!(ctx.is_empty());
}

#[test]
fn test_db_query_required_next_task() {
    let mut ctx = String::new();
    fresh(&mut ctx, "Provide me the Next task immediately");
    assert!(ctx.contains("DB_QUERY_REQUIRED"));
    assert!(ctx.contains("SESSION_PROJECT_ISOLATION"));
}

#[test]
fn test_db_query_required_whats_next() {
    let mut ctx = String::new();
    fresh(&mut ctx, "What's next?");
    assert!(ctx.contains("DB_QUERY_REQUIRED"));
}

#[test]
fn test_already_queried_suppresses_block() {
    // memory_queried=true -> the durable artifact already exists, so the gate
    // must NOT re-nag even on a status-shaped prompt (decision:rca.gate_session_amnesia).
    let mut ctx = String::new();
    append_db_query_required_with(&mut ctx, "What is the current status?", true, "kavach-rs");
    assert!(ctx.is_empty(), "queried session must suppress the block: {ctx}");
}

#[test]
fn test_slug_threads_into_block() {
    // The resolved slug must reach the emitted commands verbatim (no `<slug>`
    // placeholder leak when a real project is bound).
    let mut ctx = String::new();
    append_db_query_required_with(&mut ctx, "resume", false, "iron-will-api");
    assert!(ctx.contains("--project iron-will-api"));
    assert!(!ctx.contains("<slug>"));
}
