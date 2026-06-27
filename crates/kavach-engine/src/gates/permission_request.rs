//! `PermissionRequest` gate: fires when the permission dialog is about to show.
//!
//! Auto-allows safe tools + kavach CLI + safe cache/build rm; auto-denies
//! destructive patterns. Distinct from the `PreToolUse` permission gate.
mod allow;
mod destructive;
#[cfg(test)]
#[path = "permission_request_test.rs"]
mod tests;
use kavach_types::HookInput;
use allow::{is_kavach_command, is_safe_auto_allow, is_safe_rm_target};
use destructive::is_destructive_command;
use crate::error::EngineError;
#[expect(clippy::unnecessary_wraps, reason = "uniform gate dispatch signature")]
pub(crate) fn run(input: &HookInput) -> Result<(), EngineError> {
    let tool_name = &input.tool_name;
    if is_safe_auto_allow(tool_name) {
        drop(kavach_hook::exit_permission_request_allow(&format!(
            "auto-allowed: {tool_name}"
        )));
        return Ok(());
    }
    if tool_name == "Bash" {
        let command = input.get_string("command");
        if is_kavach_command(command) {
            drop(kavach_hook::exit_permission_request_allow(
                "kavach CLI: auto-allowed",
            ));
            return Ok(());
        }
        if is_destructive_command(command) {
            drop(kavach_hook::exit_permission_request_deny(&format!(
                "DENIED: destructive command blocked: `{command}`"
            )));
            return Ok(());
        }
        if is_safe_rm_target(command) {
            drop(kavach_hook::exit_permission_request_allow(
                "safe cleanup: cache/build dir",
            ));
            return Ok(());
        }
    }
    drop(kavach_hook::exit_permission_request_allow(&format!(
        "permitted: {tool_name}"
    )));
    Ok(())
}
