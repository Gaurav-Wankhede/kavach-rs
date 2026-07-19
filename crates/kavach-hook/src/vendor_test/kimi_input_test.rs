use crate::vendor::kimi;

#[test]
fn kimi_input_is_claude_code_compatible_with_extras_ignored() {
    let p = r#"{"session_id":"s1","cwd":"/repo","hook_event_name":"UserPromptSubmit","prompt":"hello"}"#;
    let input = kimi::lower(p).expect("kimi lowers string prompt");
    assert_eq!(input.session_id, "s1");
    assert_eq!(input.prompt, "hello");
}

#[test]
fn kimi_prompt_content_part_array_is_flattened_to_string() {
    // Kimi sends UserPromptSubmit prompt as ContentPart[], not a plain string.
    // SOURCE: https://github.com/MoonshotAI/kimi-code/issues/917
    let p = r#"{
        "session_id":"s1",
        "cwd":"/repo",
        "hook_event_name":"UserPromptSubmit",
        "prompt":[{"type":"text","text":"hello"}]
    }"#;
    let input = kimi::lower(p).expect("kimi lowers ContentPart prompt");
    assert_eq!(input.prompt, "hello");
}

#[test]
fn kimi_prompt_multiple_text_parts_joined_with_space() {
    let p = r#"{
        "session_id":"s1",
        "hook_event_name":"UserPromptSubmit",
        "prompt":[
            {"type":"text","text":"first"},
            {"type":"text","text":"second"}
        ]
    }"#;
    let input = kimi::lower(p).expect("kimi lowers multi-part prompt");
    assert_eq!(input.prompt, "first second");
}

#[test]
fn kimi_prompt_non_text_parts_are_ignored() {
    let p = r#"{
        "session_id":"s1",
        "hook_event_name":"UserPromptSubmit",
        "prompt":[
            {"type":"text","text":"use this"},
            {"type":"image","url":"http://example.com/x.png"}
        ]
    }"#;
    let input = kimi::lower(p).expect("kimi lowers mixed ContentPart prompt");
    assert_eq!(input.prompt, "use this");
}

#[test]
fn kimi_prompt_empty_content_part_array_defaults_to_empty_string() {
    let p = r#"{
        "session_id":"s1",
        "hook_event_name":"UserPromptSubmit",
        "prompt":[]
    }"#;
    let input = kimi::lower(p).expect("kimi lowers empty ContentPart prompt");
    assert_eq!(input.prompt, "");
}

#[test]
fn kimi_subagent_start_prompt_array_is_flattened() {
    let p = r#"{
        "session_id":"s1",
        "hook_event_name":"SubagentStart",
        "agent_name":"reviewer",
        "prompt":[{"type":"text","text":"review this diff"}]
    }"#;
    let input = kimi::lower(p).expect("kimi lowers SubagentStart prompt");
    assert_eq!(input.prompt, "review this diff");
    assert_eq!(input.agent_id, "reviewer");
    assert_eq!(input.agent_type, "reviewer");
}

#[test]
fn kimi_subagent_stop_response_array_is_flattened() {
    let p = r#"{
        "session_id":"s1",
        "hook_event_name":"SubagentStop",
        "agent_name":"reviewer",
        "response":[{"type":"text","text":"done"}]
    }"#;
    let input = kimi::lower(p).expect("kimi lowers SubagentStop response");
    assert_eq!(input.agent_id, "reviewer");
    assert_eq!(input.agent_type, "reviewer");
    // response lands in prompt after flattening so downstream context can read it.
    assert_eq!(input.prompt, "done");
}

#[test]
fn kimi_notification_body_is_mapped_to_message() {
    let p = r#"{
        "session_id":"s1",
        "hook_event_name":"Notification",
        "notification_type":"permission_prompt",
        "title":"Permission needed",
        "body":"Allow writing to .env?"
    }"#;
    let input = kimi::lower(p).expect("kimi lowers Notification");
    assert_eq!(input.notification_type, "permission_prompt");
    assert_eq!(input.title, "Permission needed");
    assert_eq!(input.message, "Allow writing to .env?");
}

#[test]
fn kimi_stop_failure_error_message_is_mapped_to_error() {
    let p = r#"{
        "session_id":"s1",
        "hook_event_name":"StopFailure",
        "error_type":"rpc",
        "error_message":"daemon unreachable"
    }"#;
    let input = kimi::lower(p).expect("kimi lowers StopFailure");
    assert_eq!(input.error, "daemon unreachable");
}

#[test]
fn kimi_canonical_fields_win_over_native_aliases() {
    // If both body and message are present, canonical field is kept.
    let p = r#"{
        "session_id":"s1",
        "hook_event_name":"Notification",
        "message":"canonical",
        "body":"native"
    }"#;
    let input = kimi::lower(p).expect("kimi keeps canonical field");
    assert_eq!(input.message, "canonical");
}

#[test]
fn kimi_flat_write_fields_are_lifted_into_tool_input() {
    // Some Kimi PreToolUse payloads carry tool args at the top level instead of
    // nested under tool_input. Without lifting, the pre-write gate sees an empty
    // file_path and fail-closes with [PATH_POLICY].
    let p = r#"{
        "session_id":"s1",
        "hook_event_name":"PreToolUse",
        "tool_name":"Write",
        "file_path":"src/lib.rs",
        "content":"fn main(){}"
    }"#;
    let input = kimi::lower(p).expect("kimi lowers flat Write fields");
    assert_eq!(input.get_string("file_path"), "src/lib.rs");
    assert_eq!(input.get_string("content"), "fn main(){}");
}

#[test]
fn kimi_flat_edit_fields_are_lifted_into_tool_input() {
    let p = r#"{
        "session_id":"s1",
        "hook_event_name":"PreToolUse",
        "tool_name":"Edit",
        "file_path":"src/lib.rs",
        "old_string":"old",
        "new_string":"new"
    }"#;
    let input = kimi::lower(p).expect("kimi lowers flat Edit fields");
    assert_eq!(input.get_string("file_path"), "src/lib.rs");
    assert_eq!(input.get_string("old_string"), "old");
    assert_eq!(input.get_string("new_string"), "new");
}

#[test]
fn kimi_path_alias_maps_to_file_path() {
    let p = r#"{
        "session_id":"s1",
        "hook_event_name":"PreToolUse",
        "tool_name":"Write",
        "path":"src/lib.rs",
        "contents":"fn main(){}"
    }"#;
    let input = kimi::lower(p).expect("kimi maps path alias");
    assert_eq!(input.get_string("file_path"), "src/lib.rs");
    assert_eq!(input.get_string("content"), "fn main(){}");
}

#[test]
fn kimi_nested_tool_input_still_works() {
    let p = r#"{
        "session_id":"s1",
        "hook_event_name":"PreToolUse",
        "tool_name":"Write",
        "tool_input":{"file_path":"src/lib.rs","content":"fn main(){}"}
    }"#;
    let input = kimi::lower(p).expect("kimi keeps nested tool_input");
    assert_eq!(input.get_string("file_path"), "src/lib.rs");
    assert_eq!(input.get_string("content"), "fn main(){}");
}

#[test]
fn kimi_custom_instructions_content_part_array_is_flattened() {
    // Kimi can send text-bearing fields other than prompt as ContentPart[].
    let p = r#"{
        "session_id":"s1",
        "hook_event_name":"UserPromptSubmit",
        "prompt":"hi",
        "custom_instructions":[{"type":"text","text":"be concise"}]
    }"#;
    let input = kimi::lower(p).expect("kimi lowers custom_instructions array");
    assert_eq!(input.prompt, "hi");
    assert_eq!(input.custom_instructions, "be concise");
}

#[test]
fn kimi_trigger_content_part_array_is_flattened() {
    let p = r#"{
        "session_id":"s1",
        "hook_event_name":"UserPromptSubmit",
        "prompt":"hi",
        "trigger":[{"type":"text","text":"manual"}]
    }"#;
    let input = kimi::lower(p).expect("kimi lowers trigger array");
    assert_eq!(input.trigger, "manual");
}
