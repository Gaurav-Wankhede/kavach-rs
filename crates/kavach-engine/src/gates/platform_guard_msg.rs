//! Block/advisory message builders for platform-specific pre-write guards.

use std::fmt::Write as _;

/// Build a P0 HARD BLOCK message for a named guard.
pub(crate) fn build_block(guard: &str, violations: &[(&str, &str)]) -> String {
    let mut msg = format!(
        "{guard} BLOCKED: Platform rule violations detected\n\nP0 VIOLATIONS (HARD BLOCK):\n"
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
mod tests {
    use super::*;

    #[test]
    fn build_block_formats() {
        let msg = build_block("TEST_GUARD", &[("P0_CODE", "reason text")]);
        assert!(msg.contains("TEST_GUARD BLOCKED"));
        assert!(msg.contains("P0_CODE"));
        assert!(msg.contains("reason text"));
        assert!(msg.contains("REQUIRED"));
    }

    #[test]
    fn build_block_multiple_violations() {
        let msg = build_block("X", &[("A", "reason a"), ("B", "reason b")]);
        assert!(msg.contains("A — reason a"));
        assert!(msg.contains("B — reason b"));
    }

    #[test]
    fn build_advisory_formats() {
        let msg = build_advisory("TEST_GUARD", &[("P1_CODE", "advisory text")]);
        assert!(msg.contains("[TEST_GUARD_ADVISORY]"));
        assert!(msg.contains("P1_CODE"));
        assert!(msg.contains("advisory text"));
    }

    #[test]
    fn build_advisory_empty_violations() {
        let msg = build_advisory("X", &[]);
        assert!(msg.contains("[X_ADVISORY]"));
    }
}
