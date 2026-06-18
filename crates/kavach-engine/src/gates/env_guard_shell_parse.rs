// ARCH: ShellSegmentParse
// TIME: O(n) per command — n = command length | SPACE: O(1) const separator slice
// YEAR: 2026 | SEARCHED: 2026-05
//           segments + 6 redirect tokens. Exotic shell constructs (process
//           substitution, here-strings, command groups) may be misclassified.
//           Acceptable for advisory-grade gating.
// PATTERN: lexical_segmentation | SCOPE: pre_tool_bash | CAP: AP
// FAILURE_MODE: false negative on exotic syntax → command sees first-word that
//               isn't actually executed. False positive blocks legitimate cases.
//               Both downgrade gracefully — gate is fail-open advisory.
//
// Extracted from env_guard.rs (split-env-guard-microservices roadmap, May 2026).
//
// `words` matches a segment's leading command name; `segment` handles byte-level
// redirect skipping + command-position detection.
mod segment;
mod words;

#[cfg(test)]
mod tests;

pub(crate) use segment::{is_command_position, skip_shell_redirects};
pub(crate) use words::{first_word_is, first_word_matches};
