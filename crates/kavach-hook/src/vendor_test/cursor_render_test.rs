use crate::vendor::cursor;
use kavach_types::HookResponse;

#[test]
fn cursor_pre_tool_deny_renders_permission_deny() {
    let resp = HookResponse::new_pre_tool_use_deny("BLOCKED [test]: nope");
    let out = cursor::render(&resp, "PreToolUse");
    assert!(out.contains(r#""permission":"deny""#), "must deny: {out}");
    assert!(!out.contains(r#""continue""#), "pre-tool has no continue field: {out}");
    assert!(out.contains("BLOCKED [test]"), "must carry reason: {out}");
}

#[test]
fn cursor_session_start_injects_context_as_additional_context() {
    let mut resp = HookResponse::new_approve("");
    resp.system_message =
        "[AUTONOMY_CONTRACT] claim -> implement -> 3-witness -> close\n[MISTAKE_LEDGER] do not X"
            .to_owned();
    let json = cursor::render(&resp, "SessionStart");
    assert!(
        json.contains(r#""additional_context""#),
        "session-start injects via additional_context: {json}"
    );
    assert!(json.contains("MISTAKE_LEDGER"), "context must be carried: {json}");
    assert!(
        json.contains("AUTONOMY_CONTRACT"),
        "the autonomy contract must reach the model via additional_context: {json}"
    );
}

#[test]
fn cursor_submit_emits_continue_only_no_agent_message() {
    let mut resp = HookResponse::new_approve("");
    resp.system_message = "[MISTAKE_LEDGER] do not X".to_owned();
    let json = cursor::render(&resp, "UserPromptSubmit");
    assert!(json.contains(r#""continue":true"#), "{json}");
    assert!(!json.contains("agent_message"), "submit honors no agent_message: {json}");
    assert!(
        !json.contains("MISTAKE_LEDGER"),
        "allow-path submit must NOT spam a user popup: {json}"
    );
}

#[test]
fn cursor_submit_block_surfaces_reason_in_user_message() {
    let json = cursor::render(&HookResponse::new_block("denied: bad prompt"), "UserPromptSubmit");
    assert!(json.contains(r#""continue":false"#), "{json}");
    assert!(json.contains(r#""user_message""#), "block reason rides user_message: {json}");
    assert!(json.contains("denied: bad prompt"), "{json}");
    assert!(!json.contains(r#""permission""#), "submit has no permission field: {json}");
}

#[test]
fn cursor_after_file_edit_emits_empty_object() {
    let json = cursor::render(&HookResponse::new_block("ignored"), "PostToolUse");
    assert_eq!(json, "{}", "afterFileEdit output is an empty object: {json}");
}

#[test]
fn cursor_lifecycle_hooks_emit_empty_object_not_permission_blob() {
    let resp = HookResponse::new_approve("[SUBAGENT_START] id:1");
    for event in ["PreCompact", "SubagentStart", "SubagentStop", "SessionEnd"] {
        let json = cursor::render(&resp, event);
        assert_eq!(json, "{}", "{event} must emit {{}}: {json}");
        assert!(
            !json.contains("permission"),
            "{event} must not emit permission blob: {json}"
        );
    }
}

#[test]
fn cursor_stop_block_renders_snake_case_followup_message() {
    let resp = HookResponse::new_stop_block("finish the work");
    let json = cursor::render(&resp, "Stop");
    assert!(
        json.contains("followup_message"),
        "reblock rides snake_case followup_message: {json}"
    );
    assert!(
        !json.contains("followupMessage"),
        "must NOT emit camelCase (Cursor ignores it — the loophole): {json}"
    );
    assert!(json.contains("finish the work"), "{json}");
    assert!(
        !json.contains(r#""continue""#),
        "stop hook has NO continue field per spec: {json}"
    );
    assert!(
        !json.contains(r#""permission""#),
        "stop has no permission field: {json}"
    );
}

#[test]
fn cursor_stop_clean_emits_empty_object_no_followup() {
    let resp = HookResponse::new_approve("");
    let json = cursor::render(&resp, "Stop");
    assert!(
        !json.contains("followup_message"),
        "clean stop must omit follow-up so Cursor halts: {json}"
    );
}

#[test]
fn cursor_lifecycle_hooks_emit_empty_object() {
    for event in ["PreCompact", "SubagentStart", "SubagentStop", "SessionEnd"] {
        let resp = HookResponse::new_approve("relay context queued");
        let json = cursor::render(&resp, event);
        assert_eq!(json, "{}", "{event} must be empty object: {json}");
        assert!(!json.contains("permission"), "{event} has no permission: {json}");
    }
}

#[test]
fn cursor_pre_tool_allow_carries_agent_message() {
    let resp = HookResponse::new_pre_tool_use_allow("[INTENT] type:fix");
    let json = cursor::render(&resp, "PreToolUse");
    assert!(json.contains(r#""permission":"allow""#), "{json}");
    assert!(json.contains("agent_message"), "{json}");
    assert!(json.contains("[INTENT]"), "{json}");
}

#[test]
fn cursor_pre_tool_allow_prefers_additional_context_over_allow_reason() {
    let resp = HookResponse::new_pre_tool_use_with_context(
        "allow",
        "[INTENT] type:fix risk:low complexity:simple\n[LOOP] goal:card harness:loop-until-done iter:1 done:3-witness→close→next same turn",
    );
    let json = cursor::render(&resp, "PreToolUse");
    assert!(json.contains("[INTENT]"), "shadow must reach agent_message: {json}");
    assert!(json.contains("[LOOP]"), "LOOP compact must reach agent_message: {json}");
    assert!(
        !json.contains(r#""agent_message":"allow""#),
        "boilerplate allow must not win over relay: {json}"
    );
    assert!(
        !json.contains(r#""user_message""#),
        "allow-path must not mirror relay into user_message: {json}"
    );
}
