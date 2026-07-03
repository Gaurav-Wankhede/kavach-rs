//! Dispatch correctness: unknown-gate error + representative gates from each
//! family (core / lifecycle) approve on a default input.
use super::run_gate;
use kavach_types::HookInput;

#[test]
fn test_unknown_gate() {
    let input = HookInput::default();
    let err = run_gate("nonexistent", &input).unwrap_err();
    assert!(matches!(err, crate::error::EngineError::UnknownGate(_)));
}

#[test]
fn test_lifecycle_gates_approve() {
    let input = HookInput::default();
    assert!(run_gate("session-start", &input).is_ok());
    assert!(run_gate("pre-compact", &input).is_ok());
}

#[test]
fn test_new_gates() {
    let input = HookInput::default();
    // NOTE: "stop" gate is expensive (session I/O); it has dedicated integration
    // tests. This smoke test verifies dispatch correctness for the lightweight gates.
    assert!(run_gate("notification", &input).is_ok());
    assert!(run_gate("permission", &input).is_ok());
    assert!(run_gate("post-tool-failure", &input).is_ok());
}

#[test]
fn test_new_hook_event_gates() {
    let input = HookInput::default();
    assert!(run_gate("instructions-loaded", &input).is_ok());
    assert!(run_gate("config-change", &input).is_ok());
    assert!(run_gate("worktree-create", &input).is_ok());
    assert!(run_gate("worktree-remove", &input).is_ok());
    assert!(run_gate("session-end", &input).is_ok());
}

#[test]
fn test_compact_and_elicitation_gates() {
    let input = HookInput::default();
    assert!(run_gate("post-compact", &input).is_ok());
    assert!(run_gate("elicitation", &input).is_ok());
    assert!(run_gate("elicitation-result", &input).is_ok());
}
