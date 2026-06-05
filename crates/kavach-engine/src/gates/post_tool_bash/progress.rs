//! DB-progress tracking: card STATE-TRANSITION verbs reset the stop-breaker;
//! query verbs mark memory as queried for the enforcement gates.

/// Track substantive kavach DB progress + query calls. The stop gate's
/// live-lock breaker reads `last_db_write_turn` as a "made progress" signal.
///
/// FIX [rca.stop-breaker-bookkeeping-resets-livelock]: ONLY a card STATE
/// TRANSITION (status-update / kanban-close) counts as roadmap progress. A bare
/// `kavach db write --content` (decision/research rows, or marker edits to dodge
/// dispatch) is BOOKKEEPING, not progress — counting it let a trapped agent
/// reset the breaker every turn by writing a decision row, so the live-lock
/// guard never tripped and the loop spun for ~2h. The breaker must reset only on
/// genuine card movement, never on the very writes an agent emits while stuck.
/// SOURCE: Martin Fowler — CircuitBreaker.html (only real SUCCESS resets count).
pub(super) fn track_db_progress(session: &mut kavach_session::SessionState, command: &str) {
    let is_status_update = command.contains("kavach db status-update");
    let is_kanban_close = command.contains("kavach db kanban-close");
    if is_status_update || is_kanban_close {
        session.last_db_write_turn = session.turn_count;
        session.save().ok();
    }

    // kavach db kanban/query marks memory_queried for enforcement gates.
    // ARCH: MemoryQueryTracking — pre_write_enforcement blocks implement-intent
    // without a prior db query. SOURCE: CLAUDE.md §8.2.
    if command.contains("kavach db kanban")
        || command.contains("kavach db query")
        || command.contains("kavach pipeline status")
    {
        session.mark_memory_queried();
        session.save().ok();
    }
}

#[cfg(test)]
mod tests {
    use super::track_db_progress;

    /// REGRESSION rca.stop-breaker-bookkeeping-resets-livelock: a bare
    /// `db write --content` (decision row / marker dodge) must NOT advance the
    /// live-lock progress signal — that is the bookkeeping churn a trapped agent
    /// emits, and counting it reset the breaker every turn (~2h spin).
    #[test]
    fn bare_decision_write_does_not_advance_progress() {
        let mut s = kavach_session::SessionState::default();
        s.turn_count = 42;
        track_db_progress(
            &mut s,
            "kavach db write --project p --category decision --key k --content 'note'",
        );
        assert_eq!(
            s.last_db_write_turn, 0,
            "a decision-row content write is bookkeeping, not roadmap progress"
        );
    }

    /// A card STATE TRANSITION (status-update / kanban-close) IS roadmap progress
    /// — it moves the board — so it advances the signal and resets the breaker.
    #[test]
    fn card_transition_advances_progress() {
        let mut s = kavach_session::SessionState::default();
        s.turn_count = 9;
        track_db_progress(
            &mut s,
            "kavach db status-update --project p --key k --to done",
        );
        assert_eq!(
            s.last_db_write_turn, 9,
            "status-update moves the board → progress"
        );

        s.turn_count = 11;
        track_db_progress(&mut s, "kavach db kanban-close --project p --key k");
        assert_eq!(
            s.last_db_write_turn, 11,
            "kanban-close moves the board → progress"
        );
    }
}
