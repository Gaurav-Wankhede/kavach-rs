//! Shared fail-closed block: the kanban source is unreachable, so the turn is
//! held open (an outage must never silently disable the loop).
use core::ops::ControlFlow;

/// Emit the `SOURCE_DOWN` fail-closed stop-block with a tier-specific noun
/// ("backlog", "roadmap backlog", …) and recovery instructions. Always `Break`.
pub(super) fn block(scope: &str) -> ControlFlow<()> {
    drop(kavach_hook::exit_stop_block(&format!(
        "[AUTO_CONTINUE] kanban source UNREACHABLE — cannot read the {scope} to \
         find the next task; fail-closed so the outage cannot silently disable \
         the loop.\nRECOVER: `kavach rpc` (background), then `kavach db kanban \
         --project <slug>` and resume dispatch. The loop yields only to the \
         user's `Esc`, never to a DB outage."
    )));
    ControlFlow::Break(())
}
