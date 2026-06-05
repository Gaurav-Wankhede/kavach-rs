//! Config-defined literal/regex blocklists — all P0 deny.
use super::super::decision::Decision;

/// Config-defined literal/regex blocklists (all P0 deny). `None` to fall through.
pub(super) fn config_blocklists(command: &str) -> Option<Decision> {
    if kavach_config::is_blocked_command(command) {
        return Some(Decision::Deny(format!(
            "BLOCKED: destructive command detected: `{command}`. \
             This matches the blocklist (rm -rf /, fork bombs, pipe-to-shell). \
             FIX: 1) Use targeted paths instead of root-level destructors. \
             2) Download scripts with curl -o first, review, then execute. \
             3) Use `cargo clean` or `git clean -fd` for project cleanup."
        )));
    }
    if kavach_patterns::is_blocked(command) {
        return Some(Decision::Deny(format!(
            "BLOCKED: dangerous pattern in command: `{command}`. \
             The command matches a known dangerous pattern. \
             FIX: 1) Review the command for unintended side effects. \
             2) Break compound commands into individual safe steps. \
             3) Use `eza` instead of `ls`, `bat` instead of `cat`. \
             4) Avoid piping untrusted output to shell interpreters."
        )));
    }
    if kavach_config::is_blocked_bash_pattern(command) {
        return Some(Decision::Deny(format!(
            "BLOCKED: security pattern detected: `{command}`. \
             Matches a config-defined blocked_patterns regex. \
             These patterns detect reverse shells, ssh key exfiltration, \
             and credential harvesting commands."
        )));
    }
    None
}
