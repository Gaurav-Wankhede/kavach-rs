//! Shared drained-board terminal verdict — the SINGLE source of truth both stop
//! terminals emit when the dispatch tiers find no runnable card.
//!
//! Three states hide behind "nothing dispatchable" with DIFFERENT outcomes:
//!
//! 0. The session is pinned to a lane (`KAVACH_LANE`) and its lane + the unlaned
//!    backlog are both drained → `[LANE_DRAINED]` clean stop (lane.rs). Never
//!    cross into a foreign lane; that is another session's work.
//! 1. The board still holds runnable-status cards, but EVERY one is held back by
//!    an unmet dep or an owner-gate → `[ALL_BLOCKED]` clean stop.
//! 2. The board is genuinely empty. A frozen `[PLAN]` doc MAY name an un-built
//!    next phase → a bounded `[AUTO_CONTINUE]` nudge.
//!
//! Lives HERE (`pub(in crate::gates::stop)`) so BOTH the first-pass terminal
//! (`clean_exit`) and the retry terminal emit the IDENTICAL verdict. The verdict
//! is loop-SAFE: callers emit it via `exit_stop_context` (allows the stop, no
//! hard block), so it can never spin.

mod lane;

/// The census-aware terminal context for a drained dispatch.
///
/// `open_set_census` returns `Some((runnable, blocked))` or `None` on RPC outage;
/// `None` fails closed to the nudge (never a wrong clean-stop on an unobservable
/// board).
pub(in crate::gates::stop) fn drained_terminal_context(project: &str) -> String {
    if let Some(lane_name) = lane::lane_env() {
        return lane::lane_drained_context(&lane_name);
    }
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
#[path = "drained_test.rs"]
mod tests;
