//! Guard: an in-flight bulk-mode sweep blocks clean stop until the manifest is
//! closed (`kavach bulk close`), so the conformance counters land in the audit
//! trail. SOURCE: roadmap.unit.kavach-bulk-mode acceptance #5.

use core::ops::ControlFlow;

use super::super::shared::StopCtx;

pub(crate) fn check(_ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    if let Ok(sweep_id) = std::env::var("KAVACH_BULK_SWEEP_ID")
        && !sweep_id.is_empty()
    {
        drop(kavach_hook::exit_stop_block(&format!(
            "[BULK_SWEEP_OPEN] sweep={sweep_id} still active. Clean stop \
             REFUSED until the manifest is closed. Run:\n  \
             kavach bulk close --sweep-id {sweep_id} --reason closed\n\
             (use --reason expired only if TTL fired and no further edits \
             will land). The conformance counters must be committed before \
             the session can stop."
        )));
        return ControlFlow::Break(());
    }
    ControlFlow::Continue(())
}
