use super::{HookInput, HookResponse};
use crate::EffortInput;

#[test]
fn inflight_extra_detects_monitor_array() {
    let json = r#"{"hook_event_name":"Stop","monitors":[{"id":"m1"}]}"#;
    let input: HookInput = serde_json::from_str(json).expect("parse");
    assert_eq!(input.inflight_extra_key(), Some("monitors"));
}

#[test]
fn inflight_extra_ignores_empty_and_benign() {
    let json = r#"{"monitors":[],"tags":["a","b"],"note":"x"}"#;
    let input: HookInput = serde_json::from_str(json).expect("parse");
    assert_eq!(input.inflight_extra_key(), None);
}

#[test]
fn effort_level_prefers_json_field() {
    let input = HookInput {
        effort: Some(EffortInput {
            level: "high".into(),
        }),
        ..Default::default()
    };
    assert_eq!(input.effort_level(), "high");
}

#[test]
fn effort_level_empty_json_falls_through_to_env() {
    let input = HookInput {
        effort: Some(EffortInput {
            level: String::new(),
        }),
        ..Default::default()
    };
    assert_eq!(
        input.effort_level(),
        std::env::var("CLAUDE_EFFORT").unwrap_or_default()
    );
}

#[test]
fn effort_deserializes_from_cc_wire_shape() {
    let input: HookInput =
        serde_json::from_str(r#"{"hook_event_name":"Stop","effort":{"level":"low"}}"#).unwrap();
    assert_eq!(input.effort_level(), "low");
}

#[test]
fn test_hook_input_serde_roundtrip() {
    let json = r#"{
        "session_id": "sess_abc",
        "tool_name": "Bash",
        "tool_input": {"command": "ls -la"},
        "hook_event_name": "PreToolUse",
        "cwd": "/tmp"
    }"#;
    let input: HookInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.session_id, "sess_abc");
    assert_eq!(input.tool_name, "Bash");
    assert_eq!(input.get_string("command"), "ls -la");
    assert!(input.is_event("PreToolUse"));

    let serialized = serde_json::to_string(&input).unwrap();
    let deserialized: HookInput = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.session_id, "sess_abc");
}

#[test]
fn test_hook_input_empty() {
    let input: HookInput = serde_json::from_str("{}").unwrap();
    assert_eq!(input.get_string("anything"), "");
}

#[test]
fn test_precompact_null_fields_deserialize() {
    let json = r#"{
        "session_id": "sess_x",
        "hook_event_name": "PreCompact",
        "trigger": null,
        "custom_instructions": null
    }"#;
    let input: HookInput = serde_json::from_str(json).expect("explicit-null PreCompact must parse");
    assert_eq!(input.trigger, "");
    assert_eq!(input.custom_instructions, "");
}

#[test]
fn test_hook_input_prompt_fallback() {
    let input = HookInput {
        prompt: "hello world".into(),
        ..Default::default()
    };
    assert_eq!(input.get_string("prompt"), "hello world");
}

#[test]
fn test_hook_response_approve() {
    let resp = HookResponse::new_approve("ok");
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains(r#""decision":"approve"#));
    assert!(json.contains(r#""reason":"ok"#));
}

#[test]
fn test_hook_response_block() {
    let resp = HookResponse::new_block("denied");
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains(r#""decision":"block"#));
}

#[test]
fn test_hook_specific_output_pretooluse() {
    let resp = HookResponse::new_pre_tool_use_deny("blocked cmd");
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("PreToolUse"));
    assert!(json.contains("deny"));
}

#[test]
fn test_hook_response_roundtrip_legacy() {
    let resp = HookResponse::new_modify("gate", "context here");
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: HookResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.decision, "approve");
    assert_eq!(parsed.additional_context, "context here");
}

#[test]
fn test_new_block_appends_next_action_trailer() {
    let resp = HookResponse::new_block("denied");
    assert!(resp.reason.contains("[NEXT_ACTION]"));
    assert!(resp.reason.starts_with("denied"));
}

#[test]
fn test_new_block_is_idempotent_when_trailer_present() {
    let reason = "denied\n[NEXT_ACTION] already composed";
    let resp = HookResponse::new_block(reason);
    assert_eq!(resp.reason.matches("[NEXT_ACTION]").count(), 1);
}

#[test]
fn test_new_pre_tool_use_deny_appends_next_action_trailer() {
    let resp = HookResponse::new_pre_tool_use_deny("blocked cmd");
    let reason = &resp.hook_specific_output.unwrap().permission_decision_reason;
    assert!(reason.contains("[NEXT_ACTION]"));
}

#[test]
fn test_new_user_prompt_submit_block_appends_next_action_trailer() {
    let resp = HookResponse::new_user_prompt_submit_block("prompt denied");
    assert!(resp.reason.contains("[NEXT_ACTION]"));
}

#[test]
fn test_new_stop_block_appends_next_action_trailer() {
    let resp = HookResponse::new_stop_block("stop denied");
    assert!(resp.reason.contains("[NEXT_ACTION]"));
}

#[test]
fn test_new_permission_deny_appends_next_action_trailer() {
    let resp = HookResponse::new_permission_deny("perm denied");
    assert!(resp.reason.contains("[NEXT_ACTION]"));
}
