//! `source`/`.` builtin recognition in shell command position.
use crate::gates::env_guard_shell_parse::first_word_matches;

/// Return true when `lc` contains the shell `source` builtin (or `.`) in command position.
///
/// Command position = start-of-line, or after a separator (`&&`, `||`, `;`, `|`, `(`, `{`).
/// `sqlx --source migrations_local` must NOT match — `--source` is an argv flag, not a builtin.
pub(crate) fn has_source_builtin(lc: &str) -> bool {
    first_word_matches(lc, &["source", "."])
}
