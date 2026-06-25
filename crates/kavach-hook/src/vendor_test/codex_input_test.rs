use crate::vendor::codex;

#[test]
fn codex_input_is_claude_code_compatible_with_extras_ignored() {
    let p = r#"{"session_id":"s1","turn_id":"t1","permission_mode":"plan",
                "tool_name":"Write","hook_event_name":"PreToolUse"}"#;
    let input = codex::lower(p).expect("codex lowers");
    assert_eq!(input.session_id, "s1");
    assert_eq!(input.tool_name, "Write", "CC field names pass through");
}
