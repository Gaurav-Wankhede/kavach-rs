//! Tests for OpenCode native edge.
use super::*;
use std::collections::HashMap;

#[test]
fn test_opencode_lowering() {
    let payload = r#"{
        "event": "tool.execute.before",
        "tool": "bash",
        "parameters": {"command": "cargo test"},
        "session_id": "sess-123"
    }"#;
    let input = lower(payload).expect("valid opencode payload");
    assert_eq!(input.hook_event_name, "PreToolUse");
    assert_eq!(input.tool_name, "bash");
    assert_eq!(input.session_id, "sess-123");
    let cmd = input.get_string("command");
    assert_eq!(cmd, "cargo test");
}

#[test]
fn test_opencode_render_allow_with_rewrite() {
    let mut updated = HashMap::new();
    updated.insert("command".to_owned(), serde_json::json!("rtk cargo test"));
    let resp = HookResponse::new_pre_tool_use_rewrite("rtk rewrite", updated, None);
    let rendered = render(&resp);
    assert!(rendered.contains(r#""decision":"allow""#), "{rendered}");
    assert!(rendered.contains("rtk cargo test"), "{rendered}");
}

#[test]
fn test_opencode_render_deny() {
    let resp = HookResponse::new_pre_tool_use_deny("blocked by rule");
    let rendered = render(&resp);
    assert!(rendered.contains(r#""decision":"deny""#), "{rendered}");
    assert!(rendered.contains("blocked by rule"), "{rendered}");
}
