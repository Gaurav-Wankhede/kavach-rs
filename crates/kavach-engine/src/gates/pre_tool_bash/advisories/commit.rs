//! Conventional-commit message advisory. Never blocks — commits are too
//! workflow-critical for a P0. Quote-aware so `git commit` inside another tool's
//! quoted arg does not trip it (CWE-184 over-broad-trigger).
use crate::gates::pre_tool_bash::strip_quoted_regions;

/// Recognized conventional-commit prefixes.
const PREFIXES: &[&str] = &[
    "feat", "fix", "refactor", "docs", "test", "chore", "perf", "ci", "build", "style", "spec",
    "plan", "revert",
];

/// Advisory check for conventional commit message format.
/// Returns `Some(advisory)` if the message lacks a recognized prefix.
///
/// Quote-aware: `git commit` must be command-position, not text inside another
/// tool's quoted arg. Detect on the stripped form; extract the `-m` message from
/// the ORIGINAL (the message IS the quoted text — its offset only matches `cmd`
/// when no quoted span precedes `-m `).
/// Prior FP root cause: lexical substring match instead of quote-aware
/// detection. RESEARCH: <https://cwe.mitre.org/data/definitions/184.html>
pub(in crate::gates::pre_tool_bash) fn check_commit_message(cmd: &str) -> Option<String> {
    let trimmed = cmd.trim();
    let stripped = strip_quoted_regions(trimmed);
    if !stripped.to_lowercase().contains("git commit") {
        return None;
    }
    let msg_start = trimmed.to_lowercase().find("-m ")?;
    let raw = trimmed.get(msg_start.saturating_add(3)..)?;
    let msg = raw.trim_start_matches('"').trim_start_matches('\'');
    if PREFIXES.iter().any(|p| msg.starts_with(p)) {
        return None;
    }
    Some(
        "[COMMIT_FORMAT] Advisory: commit message should use conventional commits format.\n\
         Recognized prefixes: feat | fix | refactor | docs | test | chore | perf | ci | build | style\n\
         Example: `git commit -m \"feat(auth): add PASETO token validation\"`".to_owned(),
    )
}
