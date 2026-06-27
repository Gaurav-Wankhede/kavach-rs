// WriteContext — shared context extracted once from HookInput, passed to all pipeline stages.
// Eliminates the repeated content extraction pattern across pre_write stages.
use crate::gates::pre_write_checks::{is_code_write, is_test_or_exempt};
use kavach_types::HookInput;
/// Shared context for the pre-write pipeline. Extracted once, read by all stages.
#[expect(
    clippy::struct_excessive_bools,
    reason = "four independent file categorizations: code vs test, rust vs frontend"
)]
pub(crate) struct WriteContext<'a> {
    pub file_path: &'a str,
    pub tool_name: &'a str,
    pub content: &'a str,
    pub effective_content: String,
    pub is_code: bool,
    pub is_test: bool,
    pub is_rust: bool,
    pub is_frontend: bool,
}
impl<'a> WriteContext<'a> {
    pub(crate) fn extract(input: &'a HookInput) -> Self {
        let file_path = input.get_string("file_path");
        let tool_name = &input.tool_name;
        let wr = input.get_string("content");
        let ed = input.get_string("new_string");
        let content = if wr.is_empty() { ed } else { wr };
        let effective_content = effective_body(
            tool_name,
            file_path,
            input.get_string("old_string"),
            ed,
            content,
        );
        Self {
            file_path,
            tool_name,
            content,
            effective_content,
            is_code: is_code_write(file_path),
            is_test: is_test_or_exempt(file_path),
            is_rust: kavach_patterns::is_rust_file(file_path),
            is_frontend: kavach_patterns::is_frontend_file(file_path),
        }
    }
}
/// Reconstruct the TRUE post-edit file body so downstream guards judge the
/// RESULT, not stale content. For Edit, read the current file and apply the
/// single `old_string`→`new_string` replacement (Claude Code Edit replaces the
/// unique match, touching nothing else). For Write, the body IS the content.
///
/// FAIL-CLOSED on an unverifiable Edit reconstruction. The fail-safe-defaults
/// principle (Saltzer & Schroeder 1975; reaffirmed in 2026 `DevSecOps` guidance)
/// resolves uncertainty toward the SAFE state — for a size guard that means never
/// UNDERCOUNTING. The prior behaviour returned the stale pre-edit `current` body
/// when `old_string` was empty/unmatched, so an edit that grows an already-large
/// file slipped the LOC cap. Now the worst case — whichever of {current file,
/// incoming fragment} has MORE lines — is returned, so the guard cannot be
/// under-fed. SOURCE: <https://devsecopsschool.com/blog/fail-safe-defaults/>
/// SOURCE: decision.pre-write-effective-body-post-edit.
fn effective_body(
    tool_name: &str,
    file_path: &str,
    old_string: &str,
    new_string: &str,
    content: &str,
) -> String {
    if tool_name != "Edit" || file_path.is_empty() {
        return content.to_owned();
    }
    let Ok(current) = std::fs::read_to_string(file_path) else {
        // File unreadable: cannot reconstruct → judge the incoming fragment.
        return content.to_owned();
    };
    if old_string.is_empty() || !current.contains(old_string) {
        // Edit site not locatable → true result unknown. Fail closed: return the
        // larger of {current, fragment} so the size guard sees the worst case.
        return if content.lines().count() > current.lines().count() {
            content.to_owned()
        } else {
            current
        };
    }
    current.replacen(old_string, new_string, 1)
}
#[cfg(test)]
#[path = "pre_write_context_test.rs"]
mod tests;
