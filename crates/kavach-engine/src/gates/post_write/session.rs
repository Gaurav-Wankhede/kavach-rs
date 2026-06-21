//! Stage 2: session bookkeeping on a successful write — advance turn, clear
//! failure, reset per-turn gate budget, track the modified file.
use kavach_session::SessionState;

/// Advance session state for a completed write. Resets the db-write overdue
/// clock when a file path is present (coding activity is active progress).
pub(super) fn advance_session(session: &mut SessionState, file_path: &str) {
    session.increment_turn();
    session.clear_failure();
    // Fresh per-turn gate allowance. SOURCE: roadmap.unit.gate-severity-router.
    super::super::router::reset_for_new_turn(session);
    // Parent is writing code (acting on findings) — clear any pending subagent action.
    if session.subagent_action_pending {
        session.clear_subagent_action();
    }
    if !file_path.is_empty() {
        session.add_file_modified(file_path);
        // Advance FILE-progress clock, not DB-write clock (they are distinct).
        // See decision.engine.file_vs_db_write_progress.
        session.last_write_turn = session.turn_count;
    }
}
