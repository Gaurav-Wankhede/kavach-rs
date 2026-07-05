//! Advisory message builders for the grep guard — the recursive-hang block and
//! the use-ripgrep reminder. Kept separate so the detector stays pure logic.

/// The hard-hitting performance block for a recursive `grep` missing flags.
/// `issue_list` is a comma-joined summary of the missing-flag findings.
pub(super) fn grep_performance_block(issue_list: &str) -> String {
    format!(
        "[GREP_PERFORMANCE_POLICY]\n\
         Recursive grep: {issue_list}\n\
         This causes 30+ minute hangs on large codebases.\n\n\
         USE `rg` (ripgrep) instead — 5-13x faster, auto-skips .git/binaries:\n\
         rg -n \"PATTERN\" PATH\n\n\
         TOOLBELT: kavach-engine::toolbelt::search() wraps rg with grep fallback.\n\n\
         If you MUST use Bash grep:\n\
         grep -rI --exclude-dir=.git --exclude-dir=target \
         --exclude-dir=node_modules --include='*.rs' PATTERN PATH -> retry"
    )
}

/// The softer reminder for a non-recursive `grep` — prefer ripgrep.
pub(super) fn grep_tool_reminder() -> String {
    "[GREP_TOOL_REMINDER]\n\
     You used `grep` in Bash. Use `rg` (ripgrep) instead — 5-13x faster:\n\
     rg -n \"PATTERN\" PATH\n\n\
     Benefits:\n\
     - Skips .git, target, node_modules automatically\n\
     - Skips binary files by default\n\
     - Respects .gitignore\n\
     - Parallel search across CPU cores\n\n\
     TOOLBELT: kavach-engine::toolbelt::search() wraps rg with grep fallback."
        .into()
}

/// Pointer to `kavach origin` when a search looks like a symbol-declaration lookup.
pub(crate) fn origin_pointer(symbol: &str) -> String {
    format!(
        "[KAVACH_ORIGIN_HINT]\n\
         STOP — do NOT grep for where `{symbol}` is declared. RUN this instead:\n\
         kavach origin {symbol}\n\
         It returns the exact file:line (var/fn/param/type/enum-variant/const) at zero token cost.\n\
         RESOLVE many at once: kavach origin NAME1 NAME2 ...   |   SWEEP for bug patterns: kavach hunt [PATH]"
    )
}
