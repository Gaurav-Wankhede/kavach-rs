//! Top-level Bash pre-tool router: kavach-CLI fast-path → stateless blocklist →
//! stateful advisory tail. Each stage yields one `Decision`, emitted once.
use kavach_types::HookInput;

use super::advisory_ctx;
use super::blocklist;
use super::decision::Decision;
use super::quote::is_kavach_cli;
use crate::error::EngineError;
use crate::gates::bandit::emit;
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
    decision.emit();
    Ok(())
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
