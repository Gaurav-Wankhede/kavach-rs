//! Shared drained-board terminal verdict — the SINGLE source of truth both stop
//! terminals emit when the dispatch tiers find no runnable card.
//!
//! A drained board is NOT a finished plan. Two states hide behind "nothing
//! dispatchable" and they have OPPOSITE correct outcomes:
//!
//! 1. The board still holds runnable-status cards, but EVERY one is held back by
//!    an unmet dep or an owner-gate (`AGENT_BLOCKED` / prod-deploy / CI-green).
//!    That is a DECISION hand-off, not AI work → `[ALL_BLOCKED]` naming the
//!    prerequisite, then a clean stop.
//! 2. The board is genuinely empty (zero runnable cards). A frozen `[PLAN]` doc
//!    MAY still name an un-built next phase → a bounded `[AUTO_CONTINUE]` nudge to
//!    check the plan and, if found, materialize it as a card and build it.
//!
//! Lives HERE (under `terminal`, `pub(in crate::gates::stop)`) so BOTH the
//! first-pass terminal (`clean_exit`) and the retry terminal
//! (`dispatch::retry::reblock::outcome::continue_next_phase`) emit the IDENTICAL
//! verdict. Wiring it into only one terminal was the bug: an empty board on a
//! non-re-entrant stop reached `clean_exit`, which stopped silently — never the
//! census or the plan nudge. The verdict is loop-SAFE: callers emit it via
//! `exit_stop_context` (allows the stop, no hard block), so it can never spin.

/// The census-aware terminal context for a drained dispatch: `[ALL_BLOCKED]` when
/// every remaining card is owner-gated, else the board-drained `[PLAN]` nudge.
///
/// `open_set_census` returns `Some((runnable, blocked))` or `None` on RPC outage;
/// `None` fails closed to the nudge (never a wrong clean-stop on an unobservable
/// board).
pub(in crate::gates::stop) fn drained_terminal_context(project: &str) -> String {
    if census_is_all_blocked(crate::gates::stop_dispatch::open_set_census(project)) {
        all_blocked_context()
    } else {
        board_drained_plan_context()
    }
}

/// True iff the census proves a BLOCKED remainder: at least one runnable-status
/// card AND every one of them blocked. `None` (RPC outage) → false → fail closed
/// to the PLAN nudge. An empty board (`runnable == 0`) is NOT all-blocked.
const fn census_is_all_blocked(census: Option<(u64, u64)>) -> bool {
    match census {
        Some((runnable, blocked)) => runnable > 0 && blocked == runnable,
        None => false,
    }
}

/// Case 1: every remaining runnable card is blocked/owner-gated → honest clean
/// stop. The lone owner-gated prerequisite is a DECISION for the user, not work
/// the AI can perform — surface it and stop, do NOT spin.
fn all_blocked_context() -> String {
    kavach_hook::context_block(
        "ALL_BLOCKED",
        &[
            (
                "why",
                "no card is dispatchable AND every remaining runnable card is held \
                 back by an unmet dependency or an owner-gate (AGENT_BLOCKED / prod \
                 deploy / migration-apply / CI-green / live test). None of that is \
                 work the AI can start — it is a DECISION or an external prerequisite.",
            ),
            (
                "action",
                "Clean stop. State, in one line, which owner-gated prerequisite \
                 blocks each remaining card so the owner can DECIDE or unblock it. \
                 Do NOT invent PLAN phases, do NOT re-dispatch, do NOT spin — the \
                 loop is correctly drained of AI-runnable work.",
            ),
        ],
    )
}

/// Case 2: the board is truly empty (no runnable-status cards). A frozen `[PLAN]`
/// doc MAY still carry an un-built next phase — a bounded nudge to check and, if
/// found, materialize it as a card and build it this turn.
fn board_drained_plan_context() -> String {
    kavach_hook::context_block(
        "AUTO_CONTINUE",
        &[
            (
                "why",
                "the kanban has no runnable card. If a frozen `[PLAN]` doc names an \
                 un-built next phase, that phase is AI work that was simply never \
                 written as a card; the loop should not stop while such buildable \
                 work demonstrably remains.",
            ),
            (
                "action",
                "(1) Re-read the active `[PLAN]` doc + `kavach db kanban`; (2) if — \
                 and only if — the plan names a concrete un-built next phase, WRITE \
                 it as a roadmap card (`kavach db write --category roadmap`) and \
                 START it THIS turn (you are L4 autonomous); (3) if the plan is fully \
                 built and the board is empty, this is a genuine clean stop — say so \
                 plainly. Do NOT fabricate a phase that the plan does not name.",
            ),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::{all_blocked_context, board_drained_plan_context, census_is_all_blocked};

    #[test]
    fn lone_blocked_card_is_all_blocked() {
        // The reported bug: one todo card, blocked on Windows CI → clean stop.
        assert!(census_is_all_blocked(Some((1, 1))));
    }

    #[test]
    fn every_remaining_card_blocked_is_all_blocked() {
        assert!(census_is_all_blocked(Some((3, 3))));
    }

    #[test]
    fn some_runnable_some_blocked_is_not_all_blocked() {
        // A dispatchable card exists — defer to the nudge (real work remains).
        assert!(!census_is_all_blocked(Some((3, 2))));
    }

    #[test]
    fn empty_board_is_not_all_blocked() {
        // Zero runnable cards → PLAN nudge, not an ALL_BLOCKED stop.
        assert!(!census_is_all_blocked(Some((0, 0))));
    }

    #[test]
    fn rpc_outage_fails_closed_to_nudge() {
        // None = census unobservable → never a wrong clean-stop.
        assert!(!census_is_all_blocked(None));
    }

    #[test]
    fn all_blocked_context_names_the_owner_gate() {
        let c = all_blocked_context();
        assert!(c.contains("ALL_BLOCKED"), "tag present: {c}");
        assert!(c.contains("owner-gate"), "names the prerequisite class: {c}");
    }

    #[test]
    fn plan_context_nudges_instead_of_silent_stop() {
        // The fix's core: a drained board emits the PLAN nudge, never silence.
        let c = board_drained_plan_context();
        assert!(c.contains("AUTO_CONTINUE"), "continue tag present: {c}");
        assert!(c.contains("un-built next phase"), "names the un-built work: {c}");
        assert!(
            c.contains("genuine clean stop"),
            "still allows a real stop when the plan is fully built: {c}"
        );
    }
}
