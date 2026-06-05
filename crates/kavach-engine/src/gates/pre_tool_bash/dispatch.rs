//! Top-level Bash pre-tool router: kavach-CLI fast-path → stateless blocklist →
//! stateful advisory tail. Each stage yields one `Decision`, emitted once.
use kavach_types::HookInput;

use super::advisory_ctx;
use super::blocklist;
use super::decision::Decision;
use super::quote::is_kavach_cli;
use crate::error::EngineError;
use crate::gates::bandit::{emit, shadow};
use kavach_patterns::bandit_log::BanditContext;

/// Handle Bash tool pre-check: command blocklist + pattern blocklist + advisories.
///
/// # Errors
/// Returns `Ok(())` on every path; the `Result` matches the hook dispatch
/// contract so all gate handlers share one return type.
#[expect(
    clippy::unnecessary_wraps,
    reason = "Result<(), EngineError> required by hook contract; always Ok(_)"
)]
pub(crate) fn handle_bash(input: &HookInput) -> Result<(), EngineError> {
    let command = input.get_string("command");

    // Fast-exit for kavach CLI calls — internal bookkeeping, no enforcement.
    if is_kavach_cli(command) {
        Decision::Allow(None).emit();
        return Ok(());
    }

    let decision = blocklist::check(command).unwrap_or_else(|| advisory_ctx::run(command));
    log_bandit_decision(command, &decision);
    record_canary_shadow(command, &decision);
    decision.emit();
    Ok(())
}

/// Canary SHADOW (P4): when `KAVACH_RL_CANARY` is armed, log what the RSCB-MC
/// controller WOULD decide vs the rule gate's actual verdict — without acting.
///
/// At the stateless hot path the controller has no live per-action estimates, so
/// it runs with the conservative default and abstains to `Ask` (its fail-closed
/// behavior under uncertainty). The recorded divergence — "rule acted where the
/// learned controller would defer" — is the offline signal the promotion gate
/// (`ope.evaluate` + `controller::promote`) later evaluates. No-op when disarmed.
fn record_canary_shadow(command: &str, decision: &Decision) {
    let session_id = std::env::var("KAVACH_SESSION_ID").unwrap_or_default();
    let shadow = controller_shadow_action();
    let verb = command.split_whitespace().next().unwrap_or("");
    shadow::record_shadow(&session_id, "destructive_cli_guard", verb, decision.action(), shadow);
}

/// The controller's hot-path shadow action: with no live estimates it abstains
/// to `Ask` (`GateAction::Ask`), the safe default — mapped from the OPE crate's
/// action so the two layers agree on the action vocabulary.
fn controller_shadow_action() -> kavach_patterns::bandit_log::GateAction {
    use kavach_ope::Action;
    use kavach_patterns::bandit_log::GateAction;
    let cfg = kavach_ope::controller::RiskConfig::conservative();
    match kavach_ope::controller::choose(&[], cfg) {
        Action::Allow => GateAction::Allow,
        Action::Block => GateAction::Block,
        // Ask is the abstention default; a future variant also maps to Ask (safe).
        Action::Ask | _ => GateAction::Ask,
    }
}

/// Layer-A bandit log for a Bash verdict: the action `a` + context `x` at
/// decision time, reward `None` (back-filled at the 3-witness). Pure logging,
/// fire-and-forget. `session_id` comes from the env (the pre-tool path is
/// stateless — it never loads full session state).
fn log_bandit_decision(command: &str, decision: &Decision) {
    let session_id = std::env::var("KAVACH_SESSION_ID").unwrap_or_default();
    if session_id.is_empty() {
        return;
    }
    let verb = command.split_whitespace().next().unwrap_or("");
    let diff_bytes = u32::try_from(command.len()).unwrap_or(u32::MAX);
    emit::emit_decision(
        &session_id,
        BanditContext::new("destructive_cli_guard", "Bash", verb, diff_bytes, "", 0),
        decision.action(),
        1.0,
        None,
    );
}
