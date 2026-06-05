//! Top-level Bash pre-tool router: kavach-CLI fast-path → stateless blocklist →
//! stateful advisory tail. Each stage yields one `Decision`, emitted once.
use kavach_types::HookInput;

use super::advisory_ctx;
use super::blocklist;
use super::decision::Decision;
use super::quote::is_kavach_cli;
use crate::error::EngineError;

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
    decision.emit();
    Ok(())
}
