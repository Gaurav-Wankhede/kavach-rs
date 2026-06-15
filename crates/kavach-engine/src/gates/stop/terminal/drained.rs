//! Shared drained-board terminal verdict — the SINGLE source of truth both stop
//! terminals emit when the dispatch tiers find no runnable card.
//!
//! Three states hide behind "nothing dispatchable" with DIFFERENT outcomes:
//!
//! 0. The session is pinned to a lane (`KAVACH_LANE`) and its lane + the unlaned
//!    backlog are both drained → `[LANE_DRAINED]` clean stop (lane.rs). Never
//!    cross into a foreign lane; that is another session's work.
//! 1. The board still holds runnable-status cards, but EVERY one is held back by
//!    an unmet dependency → `[ALL_BLOCKED]` clean stop.
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
    let census = crate::gates::stop_dispatch::open_set_census(project);
    // A dependency cycle is NOT a legitimate block: it is a deadlock the AI must
    // repair (break the cycle), never a clean stop. Surface it before any
    // all-blocked / plan verdict so it cannot forge a false `[ALL_BLOCKED]`.
    if census.is_some_and(|(_, _, cyclic)| cyclic > 0) {
        return cycle_deadlock_context();
    }
    if census_is_all_blocked(census) {
        all_blocked_context()
    } else {
        board_drained_plan_context()
    }
}

/// True iff the census proves a BLOCKED remainder: at least one runnable-status
/// card AND every one of them blocked by dependencies or cyclic. `None` (RPC
/// outage) → false → fail closed to the PLAN nudge. An empty board (`runnable
/// == 0`) is NOT all-blocked. A cycle is handled BEFORE this by
/// `drained_terminal_context`, so here `blocked + cyclic == runnable` still
/// counts as the all-blocked remainder.
const fn census_is_all_blocked(census: Option<(u64, u64, u64)>) -> bool {
    match census {
        Some((runnable, blocked, cyclic)) => {
            runnable > 0 && blocked.saturating_add(cyclic) == runnable
        }
        None => false,
    }
}

/// A dependency cycle holds runnable cards hostage — no card in the cycle can ever
/// become ready, so the loop would otherwise spin or falsely clean-stop. This is
/// AI-repairable work (break the cycle), so REFUSE the stop and direct the fix.
fn cycle_deadlock_context() -> String {
    kavach_hook::context_block(
        "CYCLE_DEADLOCK",
        &[
            (
                "why",
                "one or more runnable cards declare a dependency CYCLE (a card depends \
                 on itself, or A->B->A). No card in a cycle can ever satisfy its deps, \
                 so it is permanently un-dispatchable — this is a deadlock, NOT a \
                 legitimate block and NOT a clean stop.",
            ),
            (
                "action",
                "Do NOT stop. Run `kavach db kanban --format mermaid` to see the cycle, \
                 then break it: edit the offending card's `DEPENDS_ON:`/`BLOCKED_BY:` \
                 line to remove the back-edge (or re-order the work). Re-verify the \
                 census has zero cyclic cards before stopping.",
            ),
        ],
    )
}

/// Case 1: every remaining runnable card is blocked → honest clean stop. An
/// unmet dependency blocks the card, and breaking the dependency is a DECISION
/// for the user, not work the AI can perform — surface it and stop, do NOT spin.
fn all_blocked_context() -> String {
    kavach_hook::context_block(
        "ALL_BLOCKED",
        &[
            (
                "why",
                "no card is dispatchable AND every remaining runnable card is held \
                 back by an unmet dependency. None of that is work the AI can start \
                 without the prerequisite being satisfied.",
            ),
            (
                "action",
                "Clean stop. State, in one line, which unmet dependency blocks each \
                 remaining card so the owner can unblock it. Do NOT invent PLAN \
                 phases, do NOT re-dispatch, do NOT spin — the loop is correctly \
                 drained of AI-runnable work.",
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
