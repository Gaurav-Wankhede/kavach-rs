use crate::lifecycle::UserPromptSubmitOutput;
use crate::*;
use kavach_types::HookResponse;

#[test]
fn test_context_block_format() {
    let result = context_block("mygate", &[("status", "allow"), ("reason", "ok")]);
    assert_eq!(result, "[mygate]\nstatus: allow\nreason: ok\n");
}

#[test]
fn test_context_block_empty_kvs() {
    let result = context_block("gate", &[]);
    assert_eq!(result, "[gate]\n");
}

#[test]
fn test_today_format() {
    let d = today();
    assert_eq!(d.len(), 10);
    assert_eq!(
        &d.chars().nth(4).map(|c| c.to_string()).unwrap_or_default(),
        "-"
    );
    assert_eq!(
        &d.chars().nth(7).map(|c| c.to_string()).unwrap_or_default(),
        "-"
    );
}

#[test]
fn test_today_full_carries_weekday_and_iso_date() {
    // Shape: "<Weekday>, YYYY-MM-DD" — the agent-visible temporal anchor.
    let f = today_full();
    assert!(f.contains(", "), "missing weekday separator: {f}");
    // Weekday name is alphabetic and present before the comma.
    let (weekday, rest) = f.split_once(", ").expect("has separator");
    assert!(
        weekday.chars().all(char::is_alphabetic) && !weekday.is_empty(),
        "weekday not alphabetic: {weekday}"
    );
    // The ISO date trailer is exactly today()'s bare form.
    assert_eq!(rest, today(), "ISO trailer must equal today()");
}

#[test]
fn test_output_approve_json() {
    let resp = HookResponse::new_approve("test reason");
    let json = serde_json::to_string(&resp).expect("serialize");
    assert!(json.contains(r#""decision":"approve"#));
    assert!(json.contains(r#""reason":"test reason"#));
}

#[test]
fn test_output_block_json() {
    let resp = HookResponse::new_block("denied");
    let json = serde_json::to_string(&resp).expect("serialize");
    assert!(json.contains(r#""decision":"block"#));
    assert!(json.contains(r#""reason":"denied"#));
}

#[test]
fn test_output_modify_json() {
    let resp = HookResponse::new_modify("gate", "extra context");
    let json = serde_json::to_string(&resp).expect("serialize");
    assert!(json.contains(r#""decision":"approve"#));
    assert!(json.contains(r#""additionalContext":"extra context"#));
}

#[test]
fn test_output_error_json() {
    let resp = HookResponse::new_block("error: something broke");
    let json = serde_json::to_string(&resp).expect("serialize");
    assert!(json.contains("error: something broke"));
    assert!(json.contains(r#""decision":"block"#));
}

#[test]
fn test_user_prompt_submit_output_format() {
    let out = UserPromptSubmitOutput {
        hook_event_name: "UserPromptSubmit".into(),
        additional_context: "ctx".into(),
    };
    let json = serde_json::to_string(&out).expect("serialize");
    assert!(json.contains(r#""hookEventName":"UserPromptSubmit"#));
    assert!(json.contains(r#""additionalContext":"ctx"#));
}

#[test]
fn test_user_prompt_submit_silent_format() {
    let out = UserPromptSubmitOutput {
        hook_event_name: "UserPromptSubmit".into(),
        additional_context: String::new(),
    };
    let json = serde_json::to_string(&out).expect("serialize");
    assert!(json.contains(r#""additionalContext":""#));
}

#[test]
fn test_pre_tool_use_allow_json() {
    let resp = HookResponse::new_pre_tool_use_with_context("gate", "ctx");
    let json = serde_json::to_string(&resp).expect("serialize");
    assert!(json.contains("PreToolUse"));
    assert!(json.contains("allow"));
}

#[test]
fn test_pre_tool_use_deny_json() {
    let resp = HookResponse::new_pre_tool_use_deny("blocked");
    let json = serde_json::to_string(&resp).expect("serialize");
    assert!(json.contains("deny"));
    assert!(json.contains("blocked"));
}

#[test]
fn test_session_end_context_json() {
    let resp = HookResponse::new_session_end_context("bye");
    let json = serde_json::to_string(&resp).expect("serialize");
    assert!(json.contains("systemMessage"));
    assert!(json.contains("bye"));
}

#[test]
fn test_permission_allow_json() {
    let resp = HookResponse::new_permission_allow("trusted");
    let json = serde_json::to_string(&resp).expect("serialize");
    assert!(json.contains("approve"));
    assert!(json.contains("trusted"));
}

#[test]
fn test_permission_deny_json() {
    let resp = HookResponse::new_permission_deny("untrusted");
    let json = serde_json::to_string(&resp).expect("serialize");
    assert!(json.contains("block"));
    assert!(json.contains("untrusted"));
}

#[test]
fn test_context_block_no_case_change() {
    let result = context_block("MyMixedCase", &[("k", "v")]);
    assert!(result.starts_with("[MyMixedCase]\n"));
}

#[test]
fn test_module_injection_format() {
    let kvs = &[("status", "allow")];
    let module = "# loaded module content";
    let mut context = context_block("GATE", kvs);
    context.push_str("\n[MODULE:LAZY_LOADED]\n");
    context.push_str(module);
    assert!(context.contains("[MODULE:LAZY_LOADED]"));
    assert!(context.contains("# loaded module content"));
}

// CC 2.1 format tests
#[test]
fn test_cc21_pre_tool_deny_json() {
    let resp = HookResponse::new_pre_tool_use_deny("unsafe command");
    let json = serde_json::to_string(&resp).expect("serialize");
    assert!(json.contains("deny"));
    assert!(json.contains("unsafe command"));
}

#[test]
fn test_cc21_post_tool_block_json() {
    let resp = HookResponse::new_post_tool_use_block("violation", "ctx");
    let json = serde_json::to_string(&resp).expect("serialize");
    assert!(json.contains("block"));
    assert!(json.contains("PostToolUse"));
}

#[test]
fn test_cc21_prompt_context_json() {
    let resp = HookResponse::new_user_prompt_submit_context("intent data");
    let json = serde_json::to_string(&resp).expect("serialize");
    assert!(json.contains("UserPromptSubmit"));
    assert!(json.contains("intent data"));
}

#[test]
fn test_cc21_stop_block_json() {
    let resp = HookResponse::new_stop_block("unsaved work");
    let json = serde_json::to_string(&resp).expect("serialize");
    assert!(json.contains("block"));
    assert!(json.contains("unsaved work"));
}
