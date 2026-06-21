//! DB-progress tracking: card STATE-TRANSITION verbs reset the stop-breaker;
//! query verbs mark memory as queried for the enforcement gates.

/// Track substantive kavach DB progress + query calls. The stop gate's
/// live-lock breaker reads `last_db_write_turn` as a "made progress" signal.
/// SOURCE: `decision.engine.stop_breaker_card_state_transitions_only`.
pub(super) fn track_db_progress(session: &mut kavach_session::SessionState, command: &str) {
    let is_status_update = command.contains("kavach db status-update");
    let is_kanban_close = command.contains("kavach db kanban-close");
    if is_status_update || is_kanban_close {
        session.last_db_write_turn = session.turn_count;
        // SELF-CLAIM TRACKING: a self status-update->in_progress claims a card
        // without the dispatcher; mirror it into current_kanban_card so the
        // close-before-advance guard tracks it. See
        // `decision.engine.self-claim-card-tracking`.
        if is_status_update && command.contains("in_progress")
            && let Some(key) = extract_flag_value(command, "--key") {
                session.current_kanban_card = key;
            }
        session.save().ok();
    }

    // A `kavach db write --category research` satisfies TABULA_RASA gate.
    // SOURCE: decision.engine.tabula_rasa_research_row_satisfier.
    if command.contains("kavach db write") && command.contains("--category research") {
        let topic = extract_flag_value(command, "--key")
            .or_else(|| extract_flag_value(command, "--title"))
            .unwrap_or_else(|| "research".to_owned());
        session.mark_research_done_with_topic(&topic);
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

/// Extract a CLI flag's value from a command line: `--key foo` or `--key=foo`.
/// Returns the unquoted value, or `None` if the flag is absent or has no value.
/// Total + malformed-safe (no panic on a trailing bare flag).
fn extract_flag_value(command: &str, flag: &str) -> Option<String> {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    for (i, tok) in tokens.iter().enumerate() {
        if let Some(eq) = tok.strip_prefix(&format!("{flag}=")) {
            let v = eq.trim_matches(['"', '\'']);
            return (!v.is_empty()).then(|| v.to_owned());
        }
        if *tok == flag {
            let v = tokens.get(i.checked_add(1)?)?.trim_matches(['"', '\'']);
            return (!v.is_empty() && !v.starts_with("--")).then(|| v.to_owned());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{extract_flag_value, track_db_progress};

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

    /// REGRESSION `rca.tabula_rasa_research_row_not_a_satisfier`: a
    /// `db write --category research` IS a research artifact, so it satisfies the
    /// `TABULA_RASA` gate (sets `research_done` + records the key as a topic) —
    /// the dead end where an agent researched-then-persisted but stayed blocked.
    /// CRUCIAL: it must NOT advance `last_db_write_turn` (a research write proves
    /// research, but must not reset the live-lock breaker — circuit-breaker rule).
    #[test]
    fn research_row_write_satisfies_research_without_advancing_breaker() {
        let mut s = kavach_session::SessionState::default();
        s.turn_count = 7;
        track_db_progress(
            &mut s,
            "kavach db write --project p --category research --key research.foo.bar --new --content 'x'",
        );
        assert!(s.research_done, "a research-row write must satisfy tabula-rasa");
        assert!(
            s.research_topics.iter().any(|t| t == "research.foo.bar"),
            "the research key is recorded as the topic"
        );
        assert_eq!(
            s.last_db_write_turn, 0,
            "a research write proves research but must NOT reset the live-lock breaker"
        );
    }

    /// A non-research `db write` (decision/roadmap content) must NOT satisfy the
    /// research gate — only `--category research` does.
    #[test]
    fn decision_row_write_does_not_satisfy_research() {
        let mut s = kavach_session::SessionState::default();
        track_db_progress(
            &mut s,
            "kavach db write --project p --category decision --key d --content 'x'",
        );
        assert!(!s.research_done, "a decision-row write is not a research artifact");
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

    /// REGRESSION rca.card-status-drift-self-claim-untracked: a self-claim
    /// (`status-update --status in_progress`) WITHOUT the stop-gate dispatcher
    /// must point `current_kanban_card` at the claimed key, so the close-before-
    /// advance guard tracks it instead of skipping an empty card (the "task done,
    /// DB stale" drift). Mirrors the dispatcher's only-other write of that field.
    #[test]
    fn self_claim_in_progress_sets_current_card() {
        let mut s = kavach_session::SessionState::default();
        s.turn_count = 3;
        track_db_progress(
            &mut s,
            "kavach db status-update --project p --category roadmap --key roadmap.unit.42.foo --status in_progress",
        );
        assert_eq!(
            s.current_kanban_card, "roadmap.unit.42.foo",
            "self-claim must register the card for close-before-advance enforcement"
        );
    }

    /// A NON-claiming transition (status done / kanban-close) must NOT overwrite
    /// `current_kanban_card` — only an `in_progress` claim does. Otherwise closing
    /// card A would re-point the guard at A and mask a drift on the next card.
    #[test]
    fn done_transition_leaves_current_card_untouched() {
        let mut s = kavach_session::SessionState::default();
        s.current_kanban_card = "roadmap.unit.7.bar".to_owned();
        track_db_progress(
            &mut s,
            "kavach db status-update --project p --key roadmap.unit.42.foo --status done",
        );
        assert_eq!(
            s.current_kanban_card, "roadmap.unit.7.bar",
            "a done-transition must not re-claim the card slot"
        );
    }

    /// `extract_flag_value` parses both `--key v` and `--key=v` on a SINGLE token
    /// (card keys never contain spaces), strips surrounding quotes on that token,
    /// and is malformed-safe (a trailing bare flag yields None, never a panic).
    /// It is deliberately NOT a shell word-splitter — values are whitespace-free.
    #[test]
    fn extract_flag_value_handles_both_forms_and_malformed() {
        assert_eq!(
            extract_flag_value("x --key abc --y", "--key"),
            Some("abc".to_owned())
        );
        assert_eq!(
            extract_flag_value("x --key='roadmap.unit.42.foo'", "--key"),
            Some("roadmap.unit.42.foo".to_owned())
        );
        assert_eq!(extract_flag_value("x --key --status z", "--key"), None);
        assert_eq!(extract_flag_value("x --key", "--key"), None);
        assert_eq!(extract_flag_value("no flag here", "--key"), None);
    }
}
