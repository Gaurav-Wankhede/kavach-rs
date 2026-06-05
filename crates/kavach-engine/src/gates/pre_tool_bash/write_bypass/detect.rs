//! The `is_write_bypass` aggregate: detect Bash commands that modify files,
//! bypassing the Write/Edit hooks. Composes the sed/-i check, file-redirect
//! detection, `tee`, file-writing tools, and Python file/DB writes.

use super::redirect::has_file_redirect;
use super::segment::segment_first_word_is;
use super::tool_write::writes_via_tool;

#[cfg(test)]
mod tests;

/// Detect Bash commands that modify files, bypassing Write/Edit hooks.
/// Covers: `sed -i`, file redirects, `| tee`, file-writing tools, and Python
/// opening a file in write/append mode or piping to a DB client.
pub(in crate::gates::pre_tool_bash) fn is_write_bypass(cmd: &str) -> bool {
    let trimmed = cmd.trim();
    let lower = trimmed.to_lowercase();
    // `sed` must be in command position — catches `sed`/`&& sed`/`| sed`/`(sed`,
    // rejects `sedan`/`grep sediment`.
    let has_sed = segment_first_word_is(&lower, "sed");
    if has_sed && (lower.contains(" -i") || lower.contains(" -i'") || lower.contains(" -i\"")) {
        return true;
    }
    if has_file_redirect(&lower) {
        return true;
    }
    if lower.contains("| tee ") {
        return true;
    }
    if writes_via_tool(&lower) {
        return true;
    }
    let has_python = lower.starts_with("python")
        || lower.contains("python3 -c")
        || lower.contains("python -c")
        || lower.contains("/python3")
        || lower.contains("/python ")
        || lower.contains("env python");
    // Block Python piped to a DB client — bypasses SQL file hooks.
    if has_python && (lower.contains("| psql") || lower.contains("|psql")) {
        return true;
    }
    // Only block Python when it explicitly opens a file in write/append mode.
    if has_python
        && lower.contains("open(")
        && (lower.contains("'w'")
            || lower.contains("\"w\"")
            || lower.contains("'a'")
            || lower.contains("\"a\"")
            || lower.contains("'wb'")
            || lower.contains("\"wb\"")
            || lower.contains("'ab'")
            || lower.contains("\"ab\""))
    {
        return true;
    }
    false
}
