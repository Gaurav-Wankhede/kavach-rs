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

/// No card is dispatchable and no `done` card failed its witnesses → emit the
/// shared drained-board verdict (`[ALL_BLOCKED]` when every remainder is
/// owner-gated, else the `[PLAN]` nudge) and Break. The census split + the two
/// messages live in `terminal::drained` so this retry terminal and the
/// first-pass `clean_exit` terminal emit the IDENTICAL verdict — they used to
/// diverge: `clean_exit` stopped silently on an empty board, never checking the
/// plan (the reported loop bug).
pub(super) fn continue_next_phase(project: &str) -> ControlFlow<()> {
    drop(kavach_hook::exit_stop_context(
        &crate::gates::stop::terminal::drained::drained_terminal_context(project),
    ));
    ControlFlow::Break(())
}
