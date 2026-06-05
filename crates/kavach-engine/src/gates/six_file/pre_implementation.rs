// ARCH: see kavach db get --category decision --key arch.decision.fourteen_prefix_const_table
//
// Pre-implementation spec gate: production-code writes carry a six-file-context
// advisory if witnesses are missing (DEMOTED to advisory, never P0Block —
// methodology nudge, not safety). Spike mode suspends it.
mod classify;
mod context;
mod spike;

use kavach_types::HookInput;

use self::classify::is_production_code;
use self::context::resolve_project_context;
use self::spike::{active_spike, emit_spike_advisory};
use super::{report, witness};
use crate::error::EngineError;

pub(crate) fn run(input: &HookInput) -> Result<(), EngineError> {
    if !matches!(input.tool_name.as_str(), "Write" | "Edit" | "NotebookEdit") {
        return Ok(());
    }
    let path = input.get_string("file_path");
    if path.is_empty() || !is_production_code(path) {
        return Ok(());
    }
    let cwd = std::env::current_dir()
        .ok()
        .and_then(|p| p.to_str().map(str::to_owned))
        .unwrap_or_default();
    if cwd.is_empty() {
        return Ok(());
    }
    let Some(ctx) = resolve_project_context(&cwd)? else {
        return Ok(());
    };
    if let Some(reason) = active_spike(&ctx.rows) {
        emit_spike_advisory(&reason);
        return Ok(());
    }
    let result = witness::run_witness(&ctx.rows, &ctx.slug, ctx.tier);
    if result.is_clear() {
        return Ok(());
    }
    // DEMOTED to advisory (was P0Block) per roadmap.unit.gate-severity-classification.
    // Six-file presence is a methodology nudge, NOT safety. The block trained
    // agents to halt-and-ask; advisory lets the agent proceed while next-turn
    // context carries the missing-artifact list. SOURCE: code.claude.com/docs/en/hooks
    // (hookSpecificOutput additionalContext); pixelmojo 2026 quality-loop pattern.
    let block_msg = report::format_block(&result);
    drop(kavach_hook::exit_pre_tool_allow(Some(&block_msg)));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::classify::is_production_code;

    #[test]
    fn test_is_production_rs() {
        assert!(is_production_code("src/main.rs"));
        assert!(is_production_code("src/lib.rs"));
    }

    #[test]
    fn test_not_production_test() {
        assert!(!is_production_code("src/main_test.rs"));
        assert!(!is_production_code("tests/integration.rs"));
    }

    #[test]
    fn test_not_production_non_code() {
        assert!(!is_production_code("README.md"));
    }
}
