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

/// No runnable card and no witness-failing `done` card. The kanban is drained,
/// but a frozen `[PLAN]` doc may still carry an un-built next phase the AI can
/// implement — an empty BOARD is not an empty PLAN. So instead of stopping, command
/// the AI to materialize that next phase as a roadmap card and keep building; the
/// loop only truly halts once both the plan AND the board are exhausted (or the sole
/// remainder is an owner-gated prerequisite the AI literally cannot perform).
pub(super) fn continue_next_phase() -> ControlFlow<()> {
    let context = kavach_hook::context_block(
        "AUTO_CONTINUE",
        &[
            (
                "why",
                "the kanban has no dispatchable card, but an empty BOARD is not an \
                 empty PLAN. If a frozen `[PLAN]` doc names an un-built next phase, \
                 that phase is AI work that was simply never written as a card. The \
                 loop must not stop while buildable plan work remains.",
            ),
            (
                "action",
                "Do NOT stop. (1) Re-read the active `[PLAN]` doc + `kavach db \
                 kanban`; (2) if the plan names an un-built next phase, WRITE it as a \
                 roadmap card (`kavach db write --category roadmap`) and START it THIS \
                 turn — you are L4 autonomous; (3) ONLY when the plan is fully built \
                 AND the board is empty, state which owner-gated prerequisite (prod \
                 deploy / migration-apply / live test) blocks any true remainder so \
                 the owner can DECIDE — that lone case is a DECISION hand-off, never a \
                 work hand-off. Never end the turn with 'next move is yours' while a \
                 buildable phase exists.",
            ),
        ],
    );
    drop(kavach_hook::exit_stop_context(&context));
    ControlFlow::Break(())
}
