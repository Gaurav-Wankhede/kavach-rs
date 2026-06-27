// Approximate shell command segmentation for advisories (advisory-grade accuracy).
// See decision.engine.env-guard-shell-parse-arch.
//
// `words` matches a segment's leading command name; `segment` handles byte-level
// redirect skipping + command-position detection.
mod segment;
mod words;
#[cfg(test)]
#[path = "env_guard_shell_parse_test.rs"]
#[cfg(test)]
#[path = "env_guard_shell_parse_test.rs"]
mod tests;
pub(crate) use segment::{is_command_position, skip_shell_redirects};
pub(crate) use words::{first_word_is, first_word_matches};
