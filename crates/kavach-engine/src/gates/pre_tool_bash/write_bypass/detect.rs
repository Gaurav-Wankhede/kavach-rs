//! The `is_write_bypass` aggregate: detect Bash commands that modify files,
//! bypassing the Write/Edit hooks. Composes the sed/-i check, file-redirect
//! detection, `tee`, file-writing tools, and Python file/DB writes.

use super::redirect::has_file_redirect;
use super::segment::segment_first_word_is;
use super::tool_write::writes_via_tool;
#[cfg(test)]
#[path = "detect_test.rs"]
#[cfg(test)]
#[path = "detect_test.rs"]
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
    if writes_via_interpreter(&lower) {
        return true;
    }
    false
}

/// Detect a Python (or Perl) invocation that writes a file or pipes to a DB.
///
/// Covers the script-from-stdin / heredoc form `python3 - <<'PY'` and `perl -e`
/// — NOT only `-c`. The earlier denylist keyed on `python3 -c`, so a heredoc
/// (`python3 -` reading the script from stdin) laundered a file write past this
/// gate. Interpreter presence is matched in command position; the write itself
/// is matched by an `open(... 'w'/'a')` mode or a pipe to `psql`.
/// SOURCE: github.com/liberzon/claude-hooks (decompose, then match each segment)
fn writes_via_interpreter(lower: &str) -> bool {
    let has_python = segment_first_word_is(lower, "python")
        || segment_first_word_is(lower, "python3")
        || lower.contains("/python3")
        || lower.contains("/python ")
        || lower.contains("env python");
    let has_perl = segment_first_word_is(lower, "perl") || lower.contains("/perl ");
    if !has_python && !has_perl {
        return false;
    }
    // Piped to a DB client — bypasses the SQL file hooks.
    if lower.contains("| psql") || lower.contains("|psql") {
        return true;
    }
    // Python opening a file in a write/append mode (any quote/byte variant).
    if has_python && lower.contains("open(") && opens_in_write_mode(lower) {
        return true;
    }
    // `perl -i` (in-place edit) or `perl -e '... > FILE'` / `open(..,'>',..)`.
    if has_perl && (lower.contains(" -i") || lower.contains("open(") || has_file_redirect(lower)) {
        return true;
    }
    false
}

/// True when a Python `open(...)` names a write/append mode in any quote or
/// byte-string variant: `'w' "w" 'a' "a" 'wb' "wb" 'ab' "ab" 'w+' 'x'`.
fn opens_in_write_mode(lower: &str) -> bool {
    const MODES: &[&str] = &[
        "'w'", "\"w\"", "'a'", "\"a\"", "'wb'", "\"wb\"", "'ab'", "\"ab\"", "'w+'", "\"w+\"",
        "'x'", "\"x\"",
    ];
    MODES.iter().any(|m| lower.contains(m))
}
