//! Path-oriented security stages: empty path, blocked system paths, Trojan
//! Source bidi/unicode in AI-config files, agent-file + sensitive-file allows.
use super::SecurityResult;
use crate::gates::pre_write_context::WriteContext;

/// Empty `file_path` would bypass every downstream guard — hard block.
pub(super) fn empty_path(ctx: &WriteContext<'_>) -> Option<SecurityResult> {
    ctx.file_path.is_empty().then(|| {
        SecurityResult::Block(
            "[PATH_POLICY] Write/Edit called with empty file_path — all content guards \
             would be bypassed -> supply a real file_path -> retry."
                .to_owned(),
        )
    })
}

/// System paths (`/etc`, `/usr`, `/bin`, `/.ssh`, `/.aws`) are protected.
pub(super) fn system_path(ctx: &WriteContext<'_>) -> Option<SecurityResult> {
    kavach_config::is_blocked_write_path(ctx.file_path).then(|| {
        SecurityResult::Block(format!(
            "[PATH_POLICY] system path: {}. /etc /usr /bin /.ssh /.aws are protected \
             -> write to project dir or ~/.local/bin/ for user binaries -> retry.",
            ctx.file_path
        ))
    })
}

/// Trojan Source / Rules File Backdoor — bidi, zero-width, tag-block codepoints
/// in AI-config files (`.md`/`.toml`/`.mdc`), which rustc bidi lints never cover.
pub(super) fn bidi_unicode(ctx: &WriteContext<'_>) -> Option<SecurityResult> {
    if !kavach_patterns::bidi_unicode_guard::is_ai_config_path(ctx.file_path)
        || ctx.content.is_empty()
    {
        return None;
    }
    let hits = kavach_patterns::bidi_unicode_guard::scan(ctx.content);
    if hits.is_empty() {
        return None;
    }
    Some(SecurityResult::Block(
        kavach_patterns::bidi_unicode_guard::block_message(ctx.file_path, &hits),
    ))
}

/// Agent/skill/command files — allow but emit a review advisory.
pub(super) fn agent_file(ctx: &WriteContext<'_>) -> Option<SecurityResult> {
    let is_agent = ctx.file_path.contains("/.claude/agents/")
        || ctx.file_path.contains("/.claude/skills/")
        || ctx.file_path.contains("/.claude/commands/");
    is_agent.then(|| {
        SecurityResult::AllowEarly(format!(
            "[AGENT_FILE] Writing: {} — review permissionMode if present",
            ctx.file_path
        ))
    })
}

/// Sensitive files — allow but warn.
pub(super) fn sensitive_file(ctx: &WriteContext<'_>) -> Option<SecurityResult> {
    kavach_patterns::is_sensitive(ctx.file_path).then(|| {
        SecurityResult::AllowEarly(format!(
            "[SENSITIVE] Writing to sensitive file: {}",
            ctx.file_path
        ))
    })
}
