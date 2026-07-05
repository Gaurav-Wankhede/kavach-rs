//! Config-defined literal/regex blocklists — all P0 deny.
use super::super::decision::Decision;

/// Config-defined literal/regex blocklists (all P0 deny). `None` to fall through.
pub(super) fn config_blocklists(command: &str) -> Option<Decision> {
    if kavach_config::is_blocked_command(command) {
        return Some(Decision::Deny(format!(
            "[CLOUD_API_POLICY] destructive command detected: `{command}` — matches the \
             blocklist (rm -rf /, fork bombs, pipe-to-shell) -> use targeted paths instead of \
             root-level destructors, download scripts with curl -o first and review before \
             executing, or use `cargo clean`/`git clean -fd` for project cleanup -> retry."
        )));
    }
    if kavach_patterns::is_blocked(command) {
        return Some(Decision::Deny(format!(
            "[CLOUD_API_POLICY] dangerous pattern in command: `{command}` -> review the \
             command for unintended side effects, break compound commands into individual \
             safe steps, use `eza`/`bat` in place of `ls`/`cat`, and avoid piping untrusted \
             output to shell interpreters -> retry."
        )));
    }
    if kavach_config::is_blocked_bash_pattern(command) {
        return Some(Decision::Deny(format!(
            "[CLOUD_API_POLICY] security pattern detected: `{command}` — matches a \
             config-defined blocked_patterns regex (reverse shells, ssh key exfiltration, \
             credential harvesting) -> remove the pattern -> retry."
        )));
    }
    None
}
