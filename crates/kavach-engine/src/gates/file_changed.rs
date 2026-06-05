//! `FileChanged` gate — tracks external file modifications.
//! Fires when files matching hook matchers are changed outside Claude Code.

use kavach_types::HookInput;

use crate::error::EngineError;

#[expect(
    clippy::unnecessary_wraps,
    reason = "gate interface contract: run_gate expects Result<(), EngineError>"
)]
pub(crate) fn run(input: &HookInput) -> Result<(), EngineError> {
    let file_path = input.get_string("file_path");
    if file_path.is_empty() {
        drop(kavach_hook::exit_silent());
        return Ok(());
    }

    let context = kavach_hook::context_block(
        "FILE_CHANGED",
        &[("file", file_path), ("status", "external_modification")],
    );
    drop(kavach_hook::exit_notification_context(&context));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_path_is_noop() {
        let input = HookInput::default();
        assert!(run(&input).is_ok());
    }
}
