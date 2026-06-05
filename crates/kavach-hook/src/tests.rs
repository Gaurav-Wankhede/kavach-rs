#[cfg(test)]
// lib.rs declares this file as `mod tests` (via #[path]); the conventional
// inner `mod tests {}` test wrapper then trips clippy::module_inception.
// Scoped allow (NOT workspace-wide) — the lint stays active everywhere
// else; this is the well-known tests.rs false-positive, curated at the
// single site per Rust API guidelines.
#[expect(
    clippy::module_inception,
    reason = "tests.rs: module wrapper over tests—standard pattern"
)]
mod tests {
    use crate::lifecycle::UserPromptSubmitOutput;
    use crate::*;
    use kavach_types::HookResponse;
    use std::io::Cursor;

    #[test]
    fn test_read_hook_input_from_bytes() {
        let json = r#"{"session_id":"s1","tool_name":"Bash","hook_event_name":"PreToolUse"}"#;
        let cursor = Cursor::new(json.as_bytes());
        let input = read_hook_input_from(cursor).expect("should parse");
        assert_eq!(input.session_id, "s1");
        assert_eq!(input.tool_name, "Bash");
        assert!(input.is_event("PreToolUse"));
    }

    #[test]
    fn test_read_hook_input_from_empty() {
        let cursor = Cursor::new(b"{}");
        let input = read_hook_input_from(cursor).expect("should parse");
        assert_eq!(input.session_id, "");
        assert_eq!(input.tool_name, "");
    }

    #[test]
    fn test_read_hook_input_invalid_json() {
        let cursor = Cursor::new(b"not json");
        let err = read_hook_input_from(cursor).unwrap_err();
        assert!(err.contains("JSON parse error"));
    }

    #[test]
    fn explicit_null_on_a_string_field_no_longer_blocks() {
        // The exact Cursor failure: a present `null` where a String field lives.
        // Pre-scrub drops it to absent so #[serde(default)] fills "" — no
        // "invalid type: null, expected a string".
        let json = r#"{"session_id":"s1","cwd":null,"prompt":null,"tool_name":"Bash"}"#;
        let input = parse_hook_input(json).expect("null fields must not block");
        assert_eq!(input.session_id, "s1");
        assert_eq!(input.cwd, "", "null cwd defaults to empty");
        assert_eq!(input.prompt, "", "null prompt defaults to empty");
        assert_eq!(input.tool_name, "Bash");
    }

    #[test]
    fn a_cursor_shaped_payload_with_nulls_parses_without_error() {
        // A representative Cursor beforeSubmitPrompt payload (foreign field names
        // are ignored by the pivot for now; the point is it must NOT error on the
        // nulls — Wave 2 maps the names natively).
        let json = r#"{
            "conversation_id":"668320d2","generation_id":"490b90b7",
            "prompt":"do something","attachments":null,
            "hook_event_name":"beforeSubmitPrompt","workspace_roots":["/repo"],
            "cwd":null,"tool_name":null,"transcript_path":null
        }"#;
        let input = parse_hook_input(json).expect("cursor payload must parse");
        assert_eq!(input.prompt, "do something");
    }

    #[test]
    fn a_non_object_payload_still_errors() {
        // Defaulting can't recover a payload that isn't even a JSON object.
        assert!(parse_hook_input("not json").is_err());
        assert!(parse_hook_input("[1,2,3]").is_err(), "array is not a hook input");
    }

    #[test]
    fn test_read_hook_input_multiline() {
        let json = "{\n\"session_id\": \"s2\",\n\"tool_name\": \"Read\"\n}";
        let cursor = Cursor::new(json.as_bytes());
        let input = read_hook_input_from(cursor).expect("should parse multiline");
        assert_eq!(input.session_id, "s2");
        assert_eq!(input.tool_name, "Read");
    }

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
}
