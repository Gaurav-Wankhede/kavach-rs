//! Terminal stop-context emitters for the no-dispatch branch: `KEYSTONE_REPAIR`
//! (a `done` card failed witnesses → AI repair) and the board-drained branch,
//! which now `AUTO_CONTINUE`s into the next un-built `[PLAN]` phase rather than
//! clean-stopping — an empty board is not an empty plan; only a fully-built plan
//! plus an empty board (or a lone owner-gated prerequisite) is a real halt.
use core::ops::ControlFlow;

/// A `done` card failed the workspace witnesses — real, AI-fixable repair work.
/// Command the AI to fix it; never hand it back, never stop on it.
pub(super) fn keystone_repair() -> ControlFlow<()> {
    let context = kavach_hook::context_block(
        "KEYSTONE_REPAIR",
        &[
            (
                "why",
                "a `done` card FAILED the workspace witnesses (cargo check / \
                 nextest) — it is unfinished, not finished, and blocks every \
                 dependent. A witness-failing keystone IS your work, not the owner's.",
            ),
            (
                "action",
                "YOU fix the keystone THIS turn — you are L4 autonomous; the user \
                 decides direction, you do ALL the work. Steps: (1) `cargo check \
                 --workspace` + `cargo nextest run --workspace` to see the exact \
                 failure; (2) fix the code at root; (3) re-run to green; (4) the \
                 next stop auto-verifies the `done` card and dispatches dependents. \
                 NEVER hand the fix back, NEVER say 'the next move is yours'.",
            ),
        ],
    );
    drop(kavach_hook::exit_stop_context(&context));
    ControlFlow::Break(())
}

/// No card is dispatchable and no `done` card failed its witnesses. Two cases
/// hide behind that one state, and they have OPPOSITE correct outcomes:
///
/// 1. The board still holds runnable-status cards, but EVERY one is held back by
///    an unmet dep or an owner-gate (`AGENT_BLOCKED`/prod-deploy/CI-green). That
///    is not AI work — it is a DECISION hand-off. Clean-stop with `[ALL_BLOCKED]`
///    naming the prerequisite. This is the branch that stops the loop from
///    nagging forever on an owner-gated backlog.
/// 2. The board is genuinely empty (zero runnable-status cards). Only here is the
///    "an empty board is not an empty plan" nudge legitimate — emit a BOUNDED
///    `[AUTO_CONTINUE]` to check the active `[PLAN]` for an un-built next phase.
///
/// The census (`runnable`, `blocked`) is what distinguishes them. `None` =
/// RPC outage → fail closed to the nudge (never a wrong clean-stop on an
/// unobservable board).
pub(super) fn continue_next_phase(project: &str) -> ControlFlow<()> {
    if census_is_all_blocked(crate::gates::stop_dispatch::open_set_census(project)) {
        return all_blocked_stop();
    }
    board_drained_plan_nudge()
}

/// True iff the census proves a BLOCKED remainder: at least one runnable-status
/// card AND every one of them blocked. `None` (RPC outage) → false → fail closed
/// to the PLAN nudge, never a wrong clean-stop on an unobservable board. An empty
/// board (`runnable == 0`) is NOT all-blocked — it routes to the nudge.
const fn census_is_all_blocked(census: Option<(u64, u64)>) -> bool {
    match census {
        Some((runnable, blocked)) => runnable > 0 && blocked == runnable,
        None => false,
    }
}

/// Case 1: every remaining runnable card is blocked/owner-gated → honest clean
/// stop. The lone owner-gated prerequisite is a DECISION for the user, not work
/// the AI can perform — surface it and stop, do NOT spin.
fn all_blocked_stop() -> ControlFlow<()> {
    let context = kavach_hook::context_block(
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
    );
    drop(kavach_hook::exit_stop_context(&context));
    ControlFlow::Break(())
}

/// Case 2: the board is truly empty (no runnable-status cards at all). A frozen
/// `[PLAN]` doc MAY still carry an un-built next phase — a bounded nudge to check
/// and, if found, materialize it as a card and build it this turn.
fn board_drained_plan_nudge() -> ControlFlow<()> {
    let context = kavach_hook::context_block(
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
    );
    drop(kavach_hook::exit_stop_context(&context));
    ControlFlow::Break(())
}

#[cfg(test)]
mod tests {
    use super::census_is_all_blocked;

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
        // A dispatchable card exists — this branch shouldn't even be reached, but
        // if it is, do NOT clean-stop; defer to the nudge (real work remains).
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
}
