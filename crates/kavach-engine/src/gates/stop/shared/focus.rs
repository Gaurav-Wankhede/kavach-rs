//! Focus-supremacy + card-holdership predicates. Single responsibility: the two
//! pure `SessionState` checks that decide whether the user's pinned scope
//! outranks the kanban, and whether the active card could own this turn's edits.

/// True when the user's pinned scope (CLAUDE.md §FOCUS) is active AND there
/// is no in-flight bypass state — i.e. the Stop gate must NOT pull to an
/// unrelated kanban card (user intent OUTRANKS the queue). Pure over
/// `SessionState` for unit-testability. Empty `user_focus` ⇒ false ⇒ existing
/// kanban-drain behaviour is unchanged (zero regression).
pub(crate) fn user_focus_supremacy_active(session: &kavach_session::SessionState) -> bool {
    if session.user_focus.is_empty() {
        return false; // no pinned scope → kanban-drain unchanged
    }
    // Active focus, BUT in-flight bypass state must still be handled by the
    // existing terminal logic (focus reorders WORK, never weakens guards).
    let in_flight_bypass = session.has_recent_failure()
        || session.active_subagents != 0
        || (session.has_task() && session.task_status == "in_progress");
    !in_flight_bypass
}

/// True when at least one file modified THIS turn lives inside the session's
/// project `work_dir` — i.e. the active kanban card could plausibly own this
/// turn's work. False when every edit is out-of-tree (harness source under
/// `~/kavach-rs`, global config under `~/.claude`, or another repo): a card
/// rooted in this project cannot own cross-repo meta-edits, so the
/// kanban-status gate must not blame it (card mis-attribution fix). Empty
/// `work_dir` ⇒ true (cannot prove out-of-tree ⇒ preserve existing behaviour,
/// zero regression).
pub(crate) fn card_owns_any_turn_file(session: &kavach_session::SessionState) -> bool {
    if session.work_dir.is_empty() {
        return true;
    }
    let root = std::path::Path::new(&session.work_dir);
    session.files_modified_this_turn.iter().any(|f| {
        // Absolute paths: prefix-match the work_dir. Relative paths: assume
        // in-tree (they are resolved against the project cwd by the harness).
        let p = std::path::Path::new(f);
        !p.is_absolute() || p.starts_with(root)
    })
}
