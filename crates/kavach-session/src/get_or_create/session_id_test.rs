use super::{env_session_id, resolved_session_id, set_session_context};

#[test]
fn resolved_id_is_never_empty_without_env() {
    // The lease bug: with no KAVACH_SESSION_ID env AND no armed cell, the old
    // path yielded an empty holder, so todo->in_progress never persisted. The
    // binary fallback must return a non-empty per-process id so the lease claim
    // always has a holder to write.
    set_session_context("");
    if std::env::var("KAVACH_SESSION_ID")
        .unwrap_or_default()
        .is_empty()
    {
        let id = resolved_session_id();
        assert!(!id.is_empty(), "resolved id must never be empty");
        assert!(
            id.starts_with("auto-"),
            "fallback id must be process-derived: {id}"
        );
        assert_eq!(
            id,
            resolved_session_id(),
            "id must be stable across calls in one process"
        );
    }
}

#[test]
fn context_cell_surfaces_when_env_empty() {
    // The native edge arms the cell from a Cursor conversation_id (no env var).
    // With KAVACH_SESSION_ID unset, env_session_id() must surface the cell so
    // the loader routes to the per-conversation row instead of the shared
    // workdir-keyed file. Tested without touching the process env (unsafe in
    // edition 2024 and racy) — empty env is the harness default.
    set_session_context("cv-isolation-A");
    if std::env::var("KAVACH_SESSION_ID")
        .unwrap_or_default()
        .is_empty()
    {
        assert_eq!(env_session_id(), "cv-isolation-A");
    }
    // A distinct id overwrites — two conversations never collide on one cell.
    set_session_context("cv-isolation-B");
    if std::env::var("KAVACH_SESSION_ID")
        .unwrap_or_default()
        .is_empty()
    {
        assert_eq!(env_session_id(), "cv-isolation-B");
    }
    // Reset so other tests on this thread see a clean cell.
    set_session_context("");
}
