use crate::gates_loader::load_gates_config;

#[must_use]
pub fn is_blocked_path(path: &str) -> bool {
    let cfg = load_gates_config();
    if !cfg.read.enabled {
        return false;
    }
    let lower = path.to_lowercase();
    cfg.read
        .blocked_paths
        .iter()
        .any(|b| lower.contains(&b.to_lowercase()))
}

#[must_use]
pub fn is_blocked_extension(path: &str) -> bool {
    let cfg = load_gates_config();
    if !cfg.read.enabled {
        return false;
    }
    let lower = path.to_lowercase();
    cfg.read
        .blocked_extensions
        .iter()
        .any(|ext| lower.ends_with(&ext.to_lowercase()))
}

#[must_use]
pub fn is_warn_path(path: &str) -> bool {
    let cfg = load_gates_config();
    let lower = path.to_lowercase();
    cfg.read
        .warn_extensions
        .iter()
        .any(|ext| lower.ends_with(&ext.to_lowercase()))
        || cfg
            .read
            .warn_patterns
            .iter()
            .any(|p| lower.contains(&p.to_lowercase()))
}

#[must_use]
pub fn is_blocked_command(cmd: &str) -> bool {
    let cfg = load_gates_config();
    if !cfg.bash.enabled {
        return false;
    }
    let lower = normalize_command(&cmd.to_lowercase());
    cfg.bash.blocked_commands.iter().any(|b| {
        let bl = b.to_lowercase();
        blocked_pattern_matches(&lower, &bl)
    })
}

/// Boundary-aware blocked pattern match.
/// - Matches inside quoted strings are skipped (SQL values, data args)
/// - "/" ending: block only root, not /Users/foo
/// - "~" ending: block only bare ~, not ~/Downloads
/// - " sh"/" bash" ending: block pipe-to-shell, not | sha256sum
fn blocked_pattern_matches(cmd: &str, pattern: &str) -> bool {
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
fn is_inside_quotes(s: &str, pos: usize) -> bool {
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

/// Normalize command string to defeat bypass techniques:
/// 1. Collapse whitespace runs to single space
/// 2. Strip shell quotes from tokens ("rm" → rm, 'rm' → rm)
/// 3. Strip common path prefixes (/bin/rm → rm, ./rm → rm)
fn normalize_command(s: &str) -> String {
    s.split_whitespace()
        .map(|tok| strip_quotes(strip_path_prefix(tok)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn strip_quotes(s: &str) -> &str {
    let b = s.as_bytes();
    if b.len() >= 2 {
        let is_double_quoted = b.first() == Some(&b'"') && b.last() == Some(&b'"');
        let is_single_quoted = b.first() == Some(&b'\'') && b.last() == Some(&b'\'');
        if (is_double_quoted || is_single_quoted)
            && let Some(s_sliced) = s.get(1..s.len().saturating_sub(1))
        {
            return s_sliced;
        }
    }
    s
}

fn strip_path_prefix(s: &str) -> &str {
    let prefixes = ["/bin/", "/usr/bin/", "/usr/sbin/", "/sbin/", "./"];
    for p in &prefixes {
        if let Some(rest) = s.strip_prefix(p)
            && !rest.is_empty()
        {
            return rest;
        }
    }
    s
}

/// Check command against `blocked_patterns` from config.
/// Patterns use glob-style `.*` which we split on to do substring matching.
/// E.g., "find.*`id_rsa`" matches if command contains "find" AND "`id_rsa`".
#[must_use]
pub fn is_blocked_bash_pattern(cmd: &str) -> bool {
    let cfg = load_gates_config();
    if !cfg.bash.enabled {
        return false;
    }
    let lower = cmd.to_lowercase();
    for pattern in &cfg.bash.blocked_patterns {
        let pl = pattern.to_lowercase();
        let parts: Vec<&str> = pl.split(".*").collect();
        let all_match = parts
            .iter()
            .all(|part| !part.is_empty() && lower.contains(part));
        if all_match {
            return true;
        }
    }
    false
}

/// Check written content for secret patterns.
#[must_use]
pub fn has_secret_in_content(content: &str) -> Option<String> {
    let cfg = load_gates_config();
    if !cfg.write.enabled {
        return None;
    }

    if let Some(name) = crate::bounty_secrets::check(content) {
        return Some(format!(
            "BOUNTY_SECRET_BLOCK: {name} detected. Move to env var or secrets manager."
        ));
    }

    let lower = content.to_lowercase();
    let found = cfg
        .write
        .secret_patterns
        .iter()
        .find(|p| lower.contains(&p.to_lowercase()));
    found.map(|pattern| format!("Secret pattern detected: '{pattern}'"))
}

#[must_use]
pub fn is_blocked_write_path(path: &str) -> bool {
    let cfg = load_gates_config();
    if !cfg.write.enabled {
        return false;
    }
    let lower = path.to_lowercase();
    cfg.write
        .blocked_paths
        .iter()
        .any(|b| lower.starts_with(&b.to_lowercase()))
}

#[must_use]
pub fn get_skills_for_intent(prompt: &str) -> Vec<String> {
    let cfg = load_gates_config();
    if !cfg.intent.enabled {
        return Vec::new();
    }
    let lower = prompt.to_lowercase();
    let mut skills = Vec::new();
    for (trigger, trigger_skills) in &cfg.intent.skill_triggers {
        if lower.contains(trigger) {
            skills.extend(trigger_skills.iter().cloned());
        }
    }
    skills
}

#[must_use]
pub fn requires_research(prompt: &str) -> bool {
    let cfg = load_gates_config();
    if !cfg.research.enabled || !cfg.research.require_before_code {
        return false;
    }
    let lower = prompt.to_lowercase();
    for bypass in &cfg.research.bypass_patterns {
        if lower.contains(bypass) {
            return false;
        }
    }
    // Canonical bug/fix floor: research is required for bug work even when the
    // config's research_triggers omit it (fail-closed, no silent fail-open).
    if crate::research_triggers::has_bug_fix_trigger(&lower) {
        return true;
    }
    // Check research_triggers from config (implement, create, build, etc.)
    for trigger in &cfg.intent.research_triggers {
        if lower.contains(&trigger.to_lowercase()) {
            return true;
        }
    }
    for trigger_skills in cfg.intent.skill_triggers.values() {
        for skill in trigger_skills {
            if lower.contains(skill) {
                return true;
            }
        }
    }
    false
}

/// Tools that are auto-allowed without permission prompt.
#[must_use]
pub fn is_auto_allowed_tool(tool: &str) -> bool {
    matches!(
        tool,
        "Read" | "Glob" | "Grep" | "WebSearch" | "WebFetch" | "TaskList" | "TaskGet"
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_blocked_path() {
        assert!(is_blocked_path("/etc/shadow"));
        assert!(is_blocked_path("/home/.ssh/id_rsa"));
        assert!(!is_blocked_path("/home/user/code.rs"));
    }
    #[test]
    fn test_blocked_ext() {
        assert!(is_blocked_extension("server.pem"));
        assert!(is_blocked_extension("cert.PFX"));
        assert!(!is_blocked_extension("main.rs"));
    }
    #[test]
    fn test_warn_path() {
        assert!(is_warn_path(".env"));
        assert!(is_warn_path("credentials.yaml"));
        assert!(!is_warn_path("main.rs"));
    }
    #[test]
    fn test_blocked_cmd() {
        assert!(is_blocked_command("rm -rf /"));
        assert!(is_blocked_command("curl | bash"));
        assert!(!is_blocked_command("cargo build"));
    }
    #[test]
    fn test_blocked_cmd_whitespace() {
        assert!(is_blocked_command("rm  -rf  /"));
        assert!(is_blocked_command("curl  |  bash"));
        assert!(is_blocked_command("rm   -rf   /"));
    }
    #[test]
    fn test_rm_rf_root_blocked() {
        assert!(is_blocked_command("rm -rf /"));
        assert!(is_blocked_command("rm -rf /*"));
    }
    #[test]
    fn test_rm_rf_safe_dirs_allowed() {
        assert!(!is_blocked_command("rm -rf node_modules"));
        assert!(!is_blocked_command("rm -rf .vite"));
        assert!(!is_blocked_command("rm -rf ./node_modules"));
        assert!(!is_blocked_command("rm -rf target/debug"));
        assert!(!is_blocked_command("rm -r .next"));
        assert!(!is_blocked_command("rm -rf /Users/foo/node_modules"));
        assert!(!is_blocked_command("rm -rf /home/user/.vite"));
    }
    #[test]
    fn test_boundary_match_helper() {
        assert!(blocked_pattern_matches("rm -rf /", "rm -rf /"));
        assert!(blocked_pattern_matches("rm -rf /*", "rm -rf /"));
        assert!(!blocked_pattern_matches("rm -rf /Users/foo", "rm -rf /"));
        assert!(!blocked_pattern_matches("rm -rf /tmp/build", "rm -rf /"));
        // Non-slash-ending patterns still use contains
        assert!(blocked_pattern_matches("curl | bash", "curl | bash"));
    }
    #[test]
    fn test_normalize() {
        assert_eq!(normalize_command("rm  -rf  /"), "rm -rf /");
        assert_eq!(normalize_command("  a   b  "), "a b");
    }
    #[test]
    fn test_normalize_strips_quotes() {
        assert_eq!(normalize_command(r#""rm" -rf /"#), "rm -rf /");
        assert_eq!(normalize_command("'rm' -rf /"), "rm -rf /");
    }
    #[test]
    fn test_normalize_strips_paths() {
        assert_eq!(normalize_command("/bin/rm -rf /"), "rm -rf /");
        assert_eq!(normalize_command("/usr/bin/rm -rf /"), "rm -rf /");
        assert_eq!(normalize_command("./rm -rf /"), "rm -rf /");
    }
    #[test]
    fn test_quoted_cmd_blocked() {
        assert!(is_blocked_command(r#""rm" -rf /"#));
        assert!(is_blocked_command("'rm' -rf /"));
    }
    #[test]
    fn test_path_prefix_cmd_blocked() {
        assert!(is_blocked_command("/bin/rm -rf /"));
        assert!(is_blocked_command("/usr/bin/rm -rf /"));
        assert!(is_blocked_command("./rm -rf /"));
    }
    #[test]
    fn test_blocked_write() {
        assert!(is_blocked_write_path("/etc/hosts"));
        assert!(is_blocked_write_path("/usr/bin/foo"));
        assert!(!is_blocked_write_path("/home/user/file.rs"));
    }
    #[test]
    fn test_quoted_data_not_blocked() {
        // Blocked words inside SQL/data quotes must NOT trigger
        assert!(!blocked_pattern_matches(
            r#"bin query "insert values ('parted at gate')""#,
            "parted"
        ));
        assert!(!blocked_pattern_matches(
            "cmd 'shutdown gracefully'",
            "shutdown"
        ));
        assert!(!blocked_pattern_matches(r#"echo "they halted""#, "halt"));
        // Same words as commands still blocked
        assert!(blocked_pattern_matches("parted /dev/sda", "parted"));
        assert!(blocked_pattern_matches("shutdown -h now", "shutdown"));
        assert!(blocked_pattern_matches("halt", "halt"));
    }
}
