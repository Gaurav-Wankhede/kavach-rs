//! Config-defined literal/regex blocklists.
//!
//! Truly irreversible operations (reverse shells, credential exfiltration,
//! fork bombs) stay as P0 deny. Operations with safe alternatives emit
//! progressive action imperatives via Allow(Some(ctx)) — the LLM sees the
//! advisory and steers to the replacement, avoiding the infinite-loop trap
//! of Deny → retry-same → loop.
use super::super::decision::Decision;

/// Config-defined literal/regex blocklists. `None` to fall through.
pub(super) fn config_blocklists(command: &str) -> Option<Decision> {
    // --- Security patterns: reverse shells, credential exfiltration, etc. ---
    // No safe alternative exists. Hard deny.
    if kavach_config::is_blocked_bash_pattern(command) {
        return Some(Decision::Deny(format!(
            "[SECURITY_VIOLATION] security pattern detected: `{command}` — matches a \
             config-defined blocked_patterns regex (reverse shells, ssh key exfiltration, \
             credential harvesting) -> remove the pattern entirely -> retry."
        )));
    }

    // --- Destructive commands with safe alternatives: progressive action ---
    if kavach_config::is_blocked_command(command) {
        return Some(Decision::Allow(Some(format!(
            "[ADVISORY:destructive-command] `{command}` is destructive (rm -rf /, fork bombs, \
             pipe-to-shell, cloud DB deletion). ALTERNATIVES: filesystem cleanup → \
             `cargo clean` / `git clean -fd` / targeted `rm -rf <subdir>`; downloading \
             scripts → `curl -o /tmp/script.sh <url>` then review before executing; cloud \
             resources → use the provider's console for destructive ops, not CLI."
        ))));
    }

    // --- Dangerous patterns with safe alternatives: progressive action ---
    if kavach_patterns::is_blocked(command) {
        return Some(Decision::Allow(Some(format!(
            "[ADVISORY:dangerous-pattern] `{command}` matches a dangerous pattern (shutdown, \
             reboot, halt, chown -R on system dirs, dd to device, mkfs, parted, fork bombs). \
             ALTERNATIVES: power ops → let the system/package manager handle shutdowns; \
             device ops → avoid raw device writes; recursive chown → use targeted ownership \
             changes on specific files."
        ))));
    }

    None
}
