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
    let mut msg = grep_performance_block(&issues.join(", "));
    if let Some(term) = extract_symbol_term(original) {
        msg.push_str("\n\n");
        msg.push_str(&super::messages::origin_pointer(&term));
    }
    Some(msg)
}

/// Check for -I flag (skip binary files) — distinct from -i (case insensitive).
/// Uses original (non-lowered) command to distinguish -I from -i.
fn has_dash_cap_i(original: &str) -> bool {
    original
        .split_whitespace()
        .any(|arg| arg.starts_with('-') && !arg.starts_with("--") && arg.contains('I'))
}

/// Extract likely search pattern (last non-flag, non-path token).
fn extract_symbol_term(command: &str) -> Option<String> {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    for token in tokens.iter().rev() {
        if !token.starts_with('-') && !token.contains('/') && !token.contains('.') {
            let t = token.trim_matches(|c| c == '"' || c == '\'' || c == '\\');
            if is_symbol_shaped(t) {
                return Some(t.to_string());
            }
        }
    }
    None
}

fn is_symbol_shaped(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}
