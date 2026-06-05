//! Guard: iteration stale-file auto-recovery ONLY. Under the "kill blocking,
//! keep auto-continue" policy a Stop is never HALTED for an in-progress
//! iteration; this guard is retained purely for its recovery side effect —
//! clearing a recorded iteration file that no longer exists on disk (stale
//! carry-over from a crashed/prior session). It NEVER Breaks the pipeline, so a
//! live iteration falls through to the dispatch chain instead of stopping the
//! loop dead.

use core::ops::ControlFlow;

use super::super::shared::StopCtx;

pub(crate) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    if !ctx.input.stop_hook_active && !ctx.session.current_iteration_file.is_empty() {
        let file = ctx.session.current_iteration_file.clone();
        // A recorded iteration file that still exists is left untouched (no halt —
        // the dispatch chain decides what happens next). Only a STALE recording
        // (file missing) is auto-cleared so it never lingers across sessions.
        if !std::path::Path::new(&file).exists() {
            ctx.session.current_iteration_file.clear();
            ctx.session.save_or_log();
            crate::gates::event_log::log_gate_decision(
                &ctx.session.session_id,
                "stop:phase_auto_close",
                "recover",
                &format!("stale=missing file={file}"),
                &ctx.session.project,
            );
            eprintln!("[PHASE_AUTO_CLOSE] stale=missing file={file}");
        }
    }
    ControlFlow::Continue(())
}
