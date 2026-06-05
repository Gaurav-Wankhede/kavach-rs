//! Boundary-aware detection of irreversible shell commands (rm -rf /, mkfs, …).

/// True iff `cmd` matches a known destructive pattern at a word boundary.
pub(super) fn is_destructive_command(cmd: &str) -> bool {
    let lower = cmd.to_lowercase();
    let normalized = lower.split_whitespace().collect::<Vec<_>>().join(" ");
    let patterns = [
        "rm -rf /",
        "mkfs",
        "dd if=",
        "drop table",
        "drop database",
        "truncate table",
        "format c:",
        "> /dev/sda",
        ":(){ :|:& };:",
    ];
    patterns.iter().any(|p| destructive_match(&normalized, p))
}

/// Boundary-aware match for destructive command patterns.
pub(super) fn destructive_match(cmd: &str, pattern: &str) -> bool {
    let Some(pos) = cmd.find(pattern) else {
        return false;
    };
    let after = pos.saturating_add(pattern.len());
    let at_end = after >= cmd.len();
    if pattern.ends_with('/') || pattern.ends_with('~') {
        return at_end
            || cmd
                .as_bytes()
                .get(after)
                .is_some_and(|&b| matches!(b, b'*' | b' '));
    }
    if pattern.ends_with(" sh") || pattern.ends_with(" bash") {
        return at_end
            || cmd
                .as_bytes()
                .get(after)
                .is_some_and(|&b| matches!(b, b' ' | b'\'' | b'"' | b';' | b'&'));
    }
    true
}
