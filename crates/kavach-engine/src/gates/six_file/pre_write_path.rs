// ARCH: see kavach db get --category decision --key arch.decision.fourteen_prefix_const_table
//
// `classify` decides allowlisted vs forbidden paths; `message` builds the
// advisory. This hub wires them into the Write/Edit/NotebookEdit gate.
mod classify;
mod message;

#[cfg(test)]
mod tests;

use kavach_types::HookInput;

use crate::error::EngineError;

#[expect(clippy::unnecessary_wraps, reason = "uniform gate dispatch signature")]
pub(crate) fn run(input: &HookInput) -> Result<(), EngineError> {
    let tool = &input.tool_name;
    if !matches!(tool.as_str(), "Write" | "Edit" | "NotebookEdit") {
        return Ok(());
    }

    let mut path = String::new();
    input.get_string("file_path").clone_into(&mut path);
    if path.is_empty() {
        input.get_string("notebook_path").clone_into(&mut path);
    }
    if path.is_empty() {
        return Ok(());
    }

    if classify::is_allowlisted(&path) {
        return Ok(());
    }

    if classify::is_forbidden(&path) {
        // DEMOTED to advisory per roadmap.unit.gate-severity-classification.
        // Path-based six-file enforcement was P0Block; trained agents to halt and
        // re-route to spec-author agent on every doc edit. Now: advisory; agent
        // sees the canonical path mapping and decides whether to honor it.
        let reason = message::format_block(&path);
        drop(kavach_hook::exit_approve(&reason));
    }

    Ok(())
}
