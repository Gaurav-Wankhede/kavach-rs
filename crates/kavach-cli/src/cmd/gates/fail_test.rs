//! Per-vendor gate-failure policy proofs.
//!
//! Two modes: UNREADABLE input (gate never ran) and gate-RAN-but-errored
//! (RPC/DB outage). Cursor fails OPEN on unreadable input (never wedge the IDE)
//! but fails CLOSED on an enforcement-gate run-time error (A-3 safety hole).

use kavach_hook::Vendor;

use super::{fail_gate_error, fail_unreadable, is_enforcement_gate};

#[test]
fn unreadable_cursor_fails_open_with_exit_zero() {
    // Cursor's native model: an unreadable payload lets the action through.
    assert_eq!(fail_unreadable(Vendor::Cursor, "intent", "bad json"), 0);
}

#[test]
fn unreadable_codex_fails_closed_with_exit_two() {
    assert_eq!(fail_unreadable(Vendor::Codex, "pre-write", "bad json"), 2);
}

#[test]
fn unreadable_claude_code_fails_closed_with_exit_two() {
    assert_eq!(fail_unreadable(Vendor::ClaudeCode, "pre-tool", "bad json"), 2);
}

#[test]
fn gate_error_cursor_enforcement_fails_closed_exit_two() {
    // A-3: a Cursor enforcement gate that RAN and hit a DB outage must FAIL
    // CLOSED (exit 2), not fail-open like the unreadable path. This is the hole.
    assert_eq!(fail_gate_error(Vendor::Cursor, "pre-tool", "rpc down"), 2);
    assert_eq!(fail_gate_error(Vendor::Cursor, "pre-write", "rpc down"), 2);
    assert_eq!(fail_gate_error(Vendor::Cursor, "stop", "rpc down"), 2);
}

#[test]
fn gate_error_cursor_observational_fails_open_exit_zero() {
    // A non-enforcement gate's block is meaningless — fail OPEN even on error.
    assert_eq!(fail_gate_error(Vendor::Cursor, "post-tool", "rpc down"), 0);
    assert_eq!(fail_gate_error(Vendor::Cursor, "session-start", "rpc down"), 0);
}

#[test]
fn gate_error_claude_code_enforcement_fails_closed_exit_two() {
    assert_eq!(fail_gate_error(Vendor::ClaudeCode, "stop", "rpc down"), 2);
}

#[test]
fn enforcement_gates_are_exactly_pre_tool_pre_write_stop() {
    assert!(is_enforcement_gate("pre-tool"));
    assert!(is_enforcement_gate("pre-write"));
    assert!(is_enforcement_gate("stop"));
}

#[test]
fn observational_gates_are_not_enforcement() {
    for g in [
        "post-tool",
        "post-write",
        "session-start",
        "session-end",
        "intent",
        "notification",
        "pre-compact",
    ] {
        assert!(!is_enforcement_gate(g), "{g} must not be enforcement");
    }
}
