//! Stop-gate teeth for willful disobedience: REFUSE a stop whose turn dismissed a
//! fired imperative in prose without obeying it. Models `done_gaming::check`.
//! See `decision.engine.disobedience-guard`.

use core::ops::ControlFlow;

use super::shared::StopCtx;

/// Refuses the stop when this turn's message argued an imperative away (dismissal
/// vocab + imperative marker + no obey-proof). Bypass: `KAVACH_DISOBEY_BYPASS=1`.
pub(crate) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    if std::env::var_os("KAVACH_DISOBEY_BYPASS").is_some() {
        return ControlFlow::Continue(());
    }
    let msg = ctx.input.last_assistant_message.trim();
    let Some(reason) = kavach_patterns::disobedience_guard::detect_disobedience(msg) else {
        return ControlFlow::Continue(());
    };
    drop(kavach_hook::exit_stop_block(&format!(
        "[DISOBEDIENCE] (non-surrenderable) This turn {reason}. An imperative is a \
         trigger to ACT, not to argue: when a lens fires, RUN the lens detector and \
         emit `Loopholes closed:` with file:line; when research-first fires, WebSearch \
         and cite the URL; on doubt, spawn a subagent. Do the mandated action THIS \
         turn, then stop. Bypass (emergencies only): KAVACH_DISOBEY_BYPASS=1."
    )));
    ControlFlow::Break(())
}

#[cfg(test)]
#[path = "disobedience/tests.rs"]
mod tests;
