//! Native-failure-policy proofs for the gate dispatch boundary (W4).
//!
//! `run` reads real stdin, so the unit-testable contract is `fail_native`: the
//! per-vendor exit code on an unreadable payload. Cursor must fail OPEN (exit 0,
//! never wedge the IDE); Codex and Claude Code fail CLOSED (exit 2).

use super::fail_native;
use kavach_hook::Vendor;

#[test]
fn cursor_fails_open_with_exit_zero() {
    // Cursor's native model: a hook error lets the action through.
    assert_eq!(fail_native(Vendor::Cursor, "intent", "bad json"), 0);
}

#[test]
fn codex_fails_closed_with_exit_two() {
    // Codex blocks via exit code 2.
    assert_eq!(fail_native(Vendor::Codex, "pre-write", "bad json"), 2);
}

#[test]
fn claude_code_fails_closed_with_exit_two() {
    assert_eq!(fail_native(Vendor::ClaudeCode, "pre-tool", "bad json"), 2);
}
