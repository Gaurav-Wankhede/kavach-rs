//! Shared fail-closed block: the kanban source is unreachable, so a clean stop
//! is REFUSED (an outage must never silently disable the loop).
use core::ops::ControlFlow;

/// Emit the `SOURCE_DOWN` fail-closed stop-block with a tier-specific noun
/// ("backlog", "roadmap backlog", …) and recovery instructions. Always `Break`.
pub(super) fn block(scope: &str) -> ControlFlow<()> {
    drop(kavach_hook::exit_stop_block(&format!(
        "[AUTO_CONTINUE] kanban source UNREACHABLE — cannot verify the {scope} \
         is empty; clean stop REFUSED (fail-closed; this outage silently \
         disables the loop).\nRECOVER: `kavach rpc` (background), then \
         `kavach db kanban --project <slug>`. Fix the daemon before stopping."
    )));
    ControlFlow::Break(())
}
