//! `grep` command classification: is the first token `grep`, and is it a
//! recursive invocation missing the hang-avoidance flags?
use super::messages::{grep_performance_block, grep_tool_reminder};

/// Returns Some(context) with fix instructions, or None if safe.
///
/// FIX: [`SEMANTIC_STRING`] [`grep_guard.rs:15`]
/// SYMPTOM: pgrep/egrep/fgrep/zgrep/ripgrep falsely flagged as `grep`
/// WHY5: substring match violates anchor-aware-grep principle (CLAUDE.md audit P3a)
/// `ROOT_CAUSE`: contains("grep ") matches "p<grep>", "e<grep>", "rip<grep>"
/// SOLUTION: enforce word boundary by checking the first whitespace-separated token only
pub(crate) fn check_grep_command(command: &str) -> Option<String> {
    let lower = command.to_lowercase();
    let first_token = lower.split_whitespace().next().unwrap_or("");
    // Strip path prefix (e.g. /usr/bin/grep -> grep) and only match the exact command.
    let bin = first_token.rsplit('/').next().unwrap_or(first_token);
    if bin != "grep" {
        return None;
    }

    if is_recursive_grep(&lower) {
        return check_recursive_grep(&lower, command);
    }

    Some(grep_tool_reminder())
}

/// Caller must already have verified the first token is `grep` (not pgrep/ripgrep/etc).
fn is_recursive_grep(lower: &str) -> bool {
    let flags = [
        "grep -r",
        "grep -rn",
        "grep -rl",
        "grep -ri",
        "grep -rni",
        "grep -rin",
        "grep -rnil",
        "grep -rn ",
        "grep -rl ",
    ];
    flags.iter().any(|f| lower.contains(f))
}

fn check_recursive_grep(lower: &str, original: &str) -> Option<String> {
    let mut issues: Vec<&str> = Vec::new();

    if !lower.contains("--binary-files") && !has_dash_cap_i(original) {
        issues.push("scans binary files (missing -I flag)");
    }
    if !lower.contains("--exclude-dir") {
        issues.push("no --exclude-dir (scans .git, target, node_modules)");
    }
    if !lower.contains("--include") {
        issues.push("no --include filter (scans ALL file types)");
    }

    if issues.is_empty() {
        return None;
    }
    Some(grep_performance_block(&issues.join(", ")))
}

/// Check for -I flag (skip binary files) — distinct from -i (case insensitive).
/// Uses original (non-lowered) command to distinguish -I from -i.
fn has_dash_cap_i(original: &str) -> bool {
    original
        .split_whitespace()
        .any(|arg| arg.starts_with('-') && !arg.starts_with("--") && arg.contains('I'))
}
