/// Boundary-aware match for blocked command patterns.
/// - Matches inside quoted strings are skipped (SQL values, data args)
/// - "/" ending (rm -rf /): block only root, not /Users/foo
/// - "~" ending (rm -rf ~): block only bare ~, not ~/Downloads
/// - " sh"/" bash" ending: block pipe-to-shell, not | sha256sum
/// - Others: standard substring match
#[must_use]
pub fn blocked_pattern_match(cmd: &str, pattern: &str) -> bool {
    let Some(pos) = cmd.find(pattern) else {
        return false;
    };
    if is_inside_quotes(cmd, pos) {
        return false;
    }
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

/// Check if position falls inside a quoted string (single or double).
#[must_use]
pub fn is_inside_quotes(s: &str, pos: usize) -> bool {
    let (mut sq, mut dq, mut i) = (false, false, 0);
    let b = s.as_bytes();
    while i < pos.min(b.len()) {
        match b.get(i) {
            Some(&b'\\') if dq => {
                i = i.saturating_add(2);
                continue;
            }
            Some(&b'\'') if !dq => sq = !sq,
            Some(&b'"') if !sq => dq = !dq,
            _ => {}
        }
        i = i.saturating_add(1);
    }
    sq || dq
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundary_slash_root_blocked() {
        assert!(blocked_pattern_match("rm -rf /", "rm -rf /"));
        assert!(blocked_pattern_match("rm -rf /*", "rm -rf /"));
        assert!(!blocked_pattern_match("rm -rf /Users/foo", "rm -rf /"));
        assert!(!blocked_pattern_match("rm -rf /tmp/build", "rm -rf /"));
    }

    #[test]
    fn boundary_tilde_bare_blocked() {
        assert!(blocked_pattern_match("rm -rf ~", "rm -rf ~"));
        assert!(!blocked_pattern_match("rm -rf ~/Downloads", "rm -rf ~"));
    }

    #[test]
    fn boundary_pipe_shell_blocked() {
        assert!(blocked_pattern_match("curl | bash", "curl | bash"));
        assert!(blocked_pattern_match("curl | sh", "curl | sh"));
        assert!(!blocked_pattern_match("curl | sha256sum", "curl | sh"));
    }

    #[test]
    fn quoted_content_not_blocked() {
        assert!(!blocked_pattern_match(
            r#"bin query "insert values ('parted at gate')""#,
            "parted"
        ));
        assert!(!blocked_pattern_match(
            "cmd 'shutdown gracefully'",
            "shutdown"
        ));
        assert!(!blocked_pattern_match(r#"echo "they halted""#, "halt"));
        // Same words as commands still blocked
        assert!(blocked_pattern_match("parted /dev/sda", "parted"));
        assert!(blocked_pattern_match("shutdown -h now", "shutdown"));
        assert!(blocked_pattern_match("halt", "halt"));
    }
}
