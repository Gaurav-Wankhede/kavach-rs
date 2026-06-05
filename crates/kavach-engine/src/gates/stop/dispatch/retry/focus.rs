//! `USER_FOCUS` supremacy: surface the pinned scope EXACTLY ONCE per breaker
//! budget, then fall through — never reset the breaker merely because focus set.
use core::ops::ControlFlow;

use crate::gates::stop::shared::{StopCtx, user_focus_supremacy_active};

/// Surface the pinned focus once per breaker budget. `Break` if surfaced this
/// call; `Continue` once the budget is spent (falls through to forced terminal).
pub(super) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    if !(user_focus_supremacy_active(ctx.session)
        && ctx.session.stop_reblock_count < kavach_session::SessionState::max_stop_reblocks())
    {
        return ControlFlow::Continue(());
    }
    ctx.session.increment_stop_reblock();
    let attempt = ctx.session.stop_reblock_count;
    let max = kavach_session::SessionState::max_stop_reblocks();
    let focus = ctx.session.user_focus.clone();
    let context = kavach_hook::context_block(
        "USER_FOCUS_ACTIVE",
        &[
            ("focus", &focus),
            ("surfaced", &format!("{attempt}/{max}")),
            (
                "why",
                "user pinned an explicit scope — it OUTRANKS the kanban \
                 (CLAUDE.md §FOCUS). The harness must NOT auto-pull to an \
                 unrelated kanban card.",
            ),
            (
                "action",
                "complete / report the pinned focus. Surface the pivot \
                 decision to the user EXACTLY ONCE — do NOT re-emit the same \
                 pivot question (that is the forbidden §PRIME ask-loop). To \
                 work the kanban backlog the user must redirect, or clear focus \
                 with `FOCUS:CLEAR`. After this bounded budget the Stop \
                 force-terminates; the focus stays pinned and you resume work \
                 THIS turn per §FOCUS NEVER-PROPOSE-SESSION-BREAK.",
            ),
        ],
    );
    drop(kavach_hook::exit_stop_context(&context));
    ControlFlow::Break(())
}
