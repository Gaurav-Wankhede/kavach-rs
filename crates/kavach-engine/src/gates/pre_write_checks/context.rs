//! Context-string builders for the pre-write gate: the write summary fed to the
//! verification chain, and the approval TOON block emitted on allow.
use kavach_types::HookInput;
use std::fmt::Write as _;

/// Extract context from write operation for chain verification.
#[must_use]
pub(crate) fn extract_write_context(input: &HookInput) -> String {
    let file_path = input.get_string("file_path");
    let content = input.get_string("content");
    let old_str = input.get_string("old_string");
    let new_str = input.get_string("new_string");
    let mut ctx = format!("Writing to {file_path}");
    if !content.is_empty() {
        let preview: String = content.chars().take(200).collect();
        write!(ctx, " content: {preview}").ok();
    }
    if !old_str.is_empty() || !new_str.is_empty() {
        ctx.push_str(" (edit operation)");
    }
    ctx
}

/// Build the approval context TOON block for pre-write gate.
pub(crate) fn build_approval_context(
    _tool_name: &str,
    toon: &str,
    session: &mut kavach_session::SessionState,
) -> String {
    let mut context = kavach_hook::context_block("PRE_WRITE", &[("status", "allow")]);
    if !toon.is_empty() {
        context.push_str(toon);
    }
    let module_ctx = session.inject_modules_once(&["zero-stubs", "dace"]);
    context.push_str(&module_ctx);
    context
}
