//! `SessionStart` gate smoke tests — every path returns Ok.
use kavach_types::HookInput;

use super::run;

#[test]
fn test_session_start_with_model() {
    let input = HookInput {
        model: "claude-opus-4-6".into(),
        ..Default::default()
    };
    assert!(run(&input).is_ok());
}

#[test]
fn test_session_start_empty_model() {
    let input = HookInput::default();
    assert!(run(&input).is_ok());
}
