// ARCH: see kavach db get --category decision --key arch.decision.fourteen_prefix_const_table

use kavach_types::HookInput;

use crate::error::EngineError;

#[expect(clippy::unnecessary_wraps, reason = "uniform gate dispatch")]
pub(crate) fn run(input: &HookInput) -> Result<(), EngineError> {
    let tool = &input.tool_name;
    if !matches!(tool.as_str(), "Write" | "Edit" | "NotebookEdit") {
        return Ok(());
    }

    let path = input.get_string("file_path").to_owned();
    if path.is_empty() {
        return Ok(());
    }

    emit_drift_advisory(&path);
    Ok(())
}

#[expect(
    clippy::print_stderr,
    reason = "hook engine has no tracing dep; stderr is the hook log channel"
)]
fn emit_drift_advisory(path: &str) {
    let msg = format!(
        "[DRIFT_VALIDATION_TODO]\nFile written: {path}\n\
         Full AST↔spec diff validation deferred to phase 2.\n\
         Current: placeholder gate. Spec keys potentially affected: spec.*, arch.*"
    );
    eprintln!("{msg}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_implementation_returns_ok() {
        let input: HookInput = serde_json::from_str(
            r#"{"tool_name": "Write", "tool_input": {"file_path": "src/main.rs"}}"#,
        )
        .unwrap();
        assert!(run(&input).is_ok());
    }
}
