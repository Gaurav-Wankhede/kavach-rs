//! Block/advisory message builders for platform-specific pre-write guards.

use std::fmt::Write as _;

/// Build a P0 HARD BLOCK message for a named guard.
pub(crate) fn build_block(guard: &str, violations: &[(&str, &str)]) -> String {
    let mut msg = format!(
        "[{guard}_PLATFORM_POLICY] Platform rule violations detected\n\nP0 VIOLATIONS:\n"
    );
    for (code, reason) in violations {
        writeln!(msg, "  {code} — {reason}").ok();
    }
    msg.push_str(
        "\nREQUIRED: Fix all P0 violations. Data must come from API. No hardcoded values.",
    );
    msg
}

/// Build a P1 advisory message for a named guard.
pub(crate) fn build_advisory(guard: &str, advisories: &[(&str, &str)]) -> String {
    let mut msg = format!("[{guard}_ADVISORY]\n");
    for (code, reason) in advisories {
        writeln!(msg, "  {code} — {reason}").ok();
    }
    msg
}

#[cfg(test)]
#[path = "platform_guard_msg_test.rs"]
mod tests;
