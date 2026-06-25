use crate::vendor::cursor;

#[test]
fn cursor_input_maps_native_names_to_the_pivot() {
    let p = r#"{
        "conversation_id":"conv-9","prompt":"do it","workspace_roots":["/repo","/other"],
        "metadata":{"tool_name":"Bash"},"hook_event_name":"beforeShellExecution"
    }"#;
    let input = cursor::lower(p).expect("cursor lowers");
    assert_eq!(input.session_id, "conv-9", "conversation_id -> session_id");
    assert_eq!(input.prompt, "do it");
    assert_eq!(input.cwd, "/repo", "first workspace_root -> cwd");
    assert_eq!(input.tool_name, "Bash", "metadata.tool_name -> tool_name");
    assert_eq!(
        input.hook_event_name, "PreToolUse",
        "beforeShellExecution -> PreToolUse"
    );
}

#[test]
fn cursor_pretooluse_event_maps_to_canonical_pretooluse() {
    let p = r#"{
        "conversation_id":"c1","workspace_roots":["/repo"],
        "tool_name":"Write","hook_event_name":"preToolUse",
        "tool_input":{"file_path":"src/lib.rs","content":"fn main(){}"}
    }"#;
    let input = cursor::lower(p).expect("cursor lowers");
    assert_eq!(input.hook_event_name, "PreToolUse", "preToolUse -> PreToolUse");
    assert_eq!(input.tool_name, "Write");
    assert_eq!(input.get_string("file_path"), "src/lib.rs");
}

#[test]
fn cursor_shell_command_reaches_canonical_tool_input() {
    let p = r#"{
        "conversation_id":"c","metadata":{"tool_name":"Bash"},
        "hook_event_name":"beforeShellExecution","command":"rm -rf /tmp/x"
    }"#;
    let input = cursor::lower(p).expect("cursor lowers");
    assert_eq!(
        input.get_string("command"),
        "rm -rf /tmp/x",
        "cursor command must reach tool_input[command]"
    );
}

#[test]
fn cursor_input_tolerates_nulls_and_missing_fields() {
    let p = r#"{"conversation_id":null,"prompt":"hi","workspace_roots":null,"metadata":null}"#;
    let input = cursor::lower(p).expect("nulls must not block");
    assert_eq!(input.prompt, "hi");
    assert_eq!(input.session_id, "");
    assert_eq!(input.cwd, "");
}

#[test]
fn cursor_loop_count_maps_to_stop_hook_active() {
    let initial = cursor::lower(r#"{"hook_event_name":"stop","loop_count":0}"#)
        .expect("cursor lowers");
    assert!(!initial.stop_hook_active, "loop_count 0 is the initial stop");
    let reentry = cursor::lower(r#"{"hook_event_name":"stop","loop_count":3}"#)
        .expect("cursor lowers");
    assert!(reentry.stop_hook_active, "loop_count>0 is a re-entry stop");
    let absent = cursor::lower(r#"{"hook_event_name":"stop"}"#).expect("cursor lowers");
    assert!(!absent.stop_hook_active, "absent loop_count defaults to initial");
}

#[test]
fn cursor_subagent_stop_maps_to_subagent_stop_not_harness_stop() {
    let input = cursor::lower(r#"{"hook_event_name":"subagentStop","conversation_id":"c1"}"#)
        .expect("cursor lowers");
    assert_eq!(
        input.hook_event_name, "SubagentStop",
        "subagentStop must NOT map to Stop (would emit followup_message)"
    );
}
