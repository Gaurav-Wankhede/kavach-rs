use kavach_types::HookInput;

use crate::error::EngineError;

/// `ConfigChange` gate: block unauthorized config modifications.
/// Protects hooks and allowedTools from being changed during session.
#[expect(
    clippy::unnecessary_wraps,
    reason = "gate_runner::run() expects Result return for all gates"
)]
pub(crate) fn run(input: &HookInput) -> Result<(), EngineError> {
    let source = &input.source;
    let file_path = &input.file_path;

    // Policy settings cannot be blocked per CC 2.1 spec
    if source == "policy_settings" {
        drop(kavach_hook::exit_silent());
        return Ok(());
    }

    // Block modifications to hooks and allowed tools during session
    let blocked_sources = ["user_settings", "project_settings", "local_settings"];
    if blocked_sources.contains(&source.as_str()) {
        let context =
            kavach_hook::context_block("CONFIG_CHANGE", &[("source", source), ("file", file_path)]);
        drop(kavach_hook::exit_notification_context(&context));
        return Ok(());
    }

    drop(kavach_hook::exit_silent());
    Ok(())
}
