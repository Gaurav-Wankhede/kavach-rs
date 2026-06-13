use super::{env_session_id, set_session_context};

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
