//! PreWrite guard dispatch tests.
use crate::gates::pre_write_context::WriteContext;
use kavach_types::HookInput;
use std::collections::HashMap;

// TDD is ADVISORY, never a block: a production write with no test-first is ALLOWED
// through with an action-imperative nudge, never denied. SOURCE: user directive
// 2026-06-30 (gates guide, never block a reversible quality miss).
#[test]
fn tdd_missing_test_advises_but_does_not_block() {
    let mut input = HookInput::default();
    input.tool_name = "Write".into();
    input.tool_input = Some(HashMap::from([
        (
            "file_path".into(),
            serde_json::json!("crates/foo/src/widget.rs"),
        ),
        (
            "content".into(),
            serde_json::json!("pub fn build() -> i32 { 42 }\n"),
        ),
    ]));
    let ctx = WriteContext::extract(&input);
    let mut session = kavach_session::SessionState::default();
    let result = super::check(&ctx, &input, &mut session);
    assert!(
        result.block.is_none(),
        "missing test-first must NOT block — TDD is advisory"
    );
    assert!(
        result.p1_advisories.iter().any(|a| a.contains("[TDD")),
        "the test-first nudge must still surface as an advisory: {:?}",
        result.p1_advisories
    );
}
// Stage-3 guard chain must hard-block a silent-IO `let _ = fallible()` write —
// the P0 the SDLC-phase advisory must never be allowed to suppress.
#[test]
fn guard_chain_blocks_silent_io_let_underscore() {
    let mut input = HookInput::default();
    input.tool_name = "Write".into();
    input.tool_input = Some(HashMap::from([
        (
            "file_path".into(),
            serde_json::json!("crates/core/x/src/h.rs"),
        ),
        (
            "content".into(),
            serde_json::json!("pub fn f() {\n    let _ = do_io();\n}\n"),
        ),
    ]));
    let ctx = WriteContext::extract(&input);
    let mut session = kavach_session::get_or_create_session();
    let result = super::check(&ctx, &input, &mut session);
    assert!(
        result.block.is_some(),
        "silent-IO let _ = do_io() must P0-block in the guard chain"
    );
}
