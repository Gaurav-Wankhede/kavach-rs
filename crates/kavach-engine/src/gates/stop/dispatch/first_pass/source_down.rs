//! Shared fail-closed block: the kanban source is unreachable, so the turn is
//! held open (an outage must never silently disable the loop).
//!
//! A source-down has two root causes with OPPOSITE responses: a plain daemon
//! outage (restart `kavach rpc`) versus a FULL DISK (the `SurrealDB` WAL cannot
//! append). For the latter the agent must self-heal — free its own regenerable
//! build scratch and complete the write — NOT hand `rm` to the operator and
//! spin hold turns. `disk::maybe_self_heal` discriminates and emits the
//! ACT-driven directive when the DB volume is critically low.
use core::ops::ControlFlow;

use super::disk;

/// Emit the `SOURCE_DOWN` fail-closed stop-block with a tier-specific noun
/// ("backlog", "roadmap backlog", …) and recovery instructions. Always `Break`.
///
/// When the DB volume is critically low, the source-down is disk-caused: emit
/// the [`disk::self_heal_directive`] (ACT, do not hand back) instead of the
/// neutral daemon-restart text.
pub(super) fn block(scope: &str) -> ControlFlow<()> {
    if let Some(directive) = disk::maybe_self_heal() {
        drop(kavach_hook::exit_stop_block(&directive));
        return ControlFlow::Break(());
    }
    drop(kavach_hook::exit_stop_block(&format!(
        "[AUTO_CONTINUE] kanban source UNREACHABLE — cannot read the {scope} to \
         find the next task; fail-closed so the outage cannot silently disable \
         the loop.\nRECOVER: `kavach rpc` (background), then `kavach db kanban \
         --project <slug>` and resume dispatch. The loop yields only to the \
         user's `Esc`, never to a DB outage."
    )));
    ControlFlow::Break(())
}
