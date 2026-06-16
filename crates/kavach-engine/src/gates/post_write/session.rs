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
        // A file edit advances the FILE-progress clock (`last_write_turn`) — NOT
        // the DB-write clock. Conflating them (the old `last_db_write_turn = ...`)
        // made a mere edit masquerade as a card status-update, so the close-before-
        // advance check believed the DB was current and never demanded a
        // `kavach db status-update` — the card silently drifted while work was
        // "done". `last_write_turn` keeps the live-lock breaker reset on code
        // progress (markers::has_progress_since_last_stop reads BOTH clocks); ONLY
        // a real card transition (post_tool_bash::track_db_progress) may set
        // `last_db_write_turn`. SOURCE: rca.card-status-drift-file-edit-masks-db.
        session.last_write_turn = session.turn_count;
    }
}
