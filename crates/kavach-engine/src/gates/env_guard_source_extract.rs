// ARCH: SourceBuiltinExtraction
// TIME: O(n*k) — n = command length, k = needles (2: 'source ' and '. ') | SPACE: O(n) lowercase copy
// YEAR: 2026 | SEARCHED: 2026-05
//           Approximate but distinguishes `source .env && X` from `sqlx --source X`.
// PATTERN: lexical_position_filter | SCOPE: pre_tool_bash | CAP: AP
// FAILURE_MODE: false negative if downstream uses unusual separator (e.g. backgrounding `&`);
//               check_env_value_read falls through to its other branches.
//
// Extracted from env_guard.rs (split-env-guard-microservices roadmap, May 2026).
mod builtin;
mod extract;
mod offset;
mod scan;

#[cfg(test)]
mod tests;

pub(crate) use builtin::has_source_builtin;
pub(crate) use extract::extract_post_source_command;
