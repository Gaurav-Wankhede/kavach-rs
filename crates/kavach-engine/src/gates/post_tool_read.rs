use kavach_types::HookInput;

use crate::error::EngineError;

/// Extract skill name from a local skill SKILL.md path and record invocation.
/// `~/.claude/skills/testing/SKILL.md` → records "testing"
fn extract_local_skill_name(path: &str, session: &mut kavach_session::SessionState) {
    // Path form: .../skills/<skill-name>/SKILL.md
    let Some(before_skill_md) = path.strip_suffix("/SKILL.md") else {
        return;
    };
    let Some(slash) = before_skill_md.rfind('/') else {
        return;
    };
    let Some(skill_name) = before_skill_md.get(slash.saturating_add(1)..) else {
        return;
    };
    if !skill_name.is_empty() {
        session.record_skill_invoked(skill_name);
    }
}

/// Handle read done: warn on sensitive files + track duplicates.
///
/// # Errors
/// Returns `Ok(())` on every path; the `Result` matches the `post_tool::run`
/// match dispatch so all per-tool handlers share one return type.
#[expect(
    clippy::unnecessary_wraps,
    reason = "signature fixed by the post_tool::run match dispatch: every per-tool handler returns Result<(), EngineError>"
)]
pub(crate) fn handle(
    input: &HookInput,
    session: &mut kavach_session::SessionState,
) -> Result<(), EngineError> {
    let file_path = input.get_string("file_path");
    let tool_name = &input.tool_name;
    let target = if file_path.is_empty() {
        input.get_string("pattern")
    } else {
        file_path
    };

    let mut parts: Vec<String> = Vec::new();

    // Check for sensitive file access
    if !file_path.is_empty() && kavach_patterns::is_sensitive(file_path) {
        parts.push(kavach_hook::context_block(
            "POST_TOOL:READ",
            &[("file", file_path), ("reason", "sensitive file was read")],
        ));
    }

    // Track file reads for attention dilution detection
    if tool_name == "Read" {
        session.increment_files_read();
    }

    // Satisfy skill gate when a local skill's SKILL.md is read directly.
    // Local skills (e.g. ~/.claude/skills/testing/SKILL.md) cannot be loaded via
    // the Skill tool — reading their SKILL.md is the only available invocation path.
    if tool_name == "Read" && file_path.contains("/skills/") && file_path.ends_with("SKILL.md") {
        extract_local_skill_name(file_path, session);
    }

    // Check for attention dilution (12+ files in one pass)
    if let Some(attention_warn) = super::attention_guard::check_attention(session) {
        parts.push(attention_warn);
    }

    // Check for duplicate reads/searches
    if let Some(dup_warn) =
        super::duplicate_tool_guard::check_duplicate_tool(session, tool_name, target)
    {
        parts.push(dup_warn);
    }
    super::duplicate_tool_guard::record_tool_call(session, tool_name, target);

    if parts.is_empty() {
        drop(kavach_hook::exit_silent());
    } else {
        drop(kavach_hook::exit_post_tool_context(&parts.join("\n\n")));
    }

    Ok(())
}
