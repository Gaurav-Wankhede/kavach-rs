use std::io::Cursor;
use crate::*;

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
    assert!(
        parse_hook_input("[1,2,3]").is_err(),
        "array of primitives is not a hook input"
    );
}

#[test]
fn sequence_input_with_object_is_handled() {
    // Kimi may send a sequence with a single object inside
    let json = r#"[{"session_id":"s1","tool_name":"Bash","hook_event_name":"PreToolUse"}]"#;
    let input = parse_hook_input(json).expect("sequence with object should parse");
    assert_eq!(input.session_id, "s1");
    assert_eq!(input.tool_name, "Bash");
    assert!(input.is_event("PreToolUse"));
}

#[test]
fn sequence_input_with_nulls_is_handled() {
    // Kimi may send a sequence with null fields
    let json = r#"[{"session_id":"s1","cwd":null,"prompt":null,"tool_name":"Bash"}]"#;
    let input = parse_hook_input(json).expect("sequence with nulls should parse");
    assert_eq!(input.session_id, "s1");
    assert_eq!(input.cwd, "", "null cwd defaults to empty");
    assert_eq!(input.prompt, "", "null prompt defaults to empty");
    assert_eq!(input.tool_name, "Bash");
}

#[test]
fn empty_sequence_errors() {
    // Empty array should error
    let json = r#"[]"#;
    let err = parse_hook_input(json).unwrap_err();
    assert!(err.contains("empty array"));
}

#[test]
fn sequence_with_non_object_errors() {
    // Array with non-object elements should error
    let json = r#"[{"session_id":"s1"}, "not an object"]"#;
    let input = parse_hook_input(json);
    // Should either succeed with first element or error if first is invalid
    if let Ok(i) = input {
        assert_eq!(i.session_id, "s1");
    } else {
        assert!(input.unwrap_err().contains("not an object"));
    }
}

#[test]
fn test_read_hook_input_multiline() {
    let json = "{\n\"session_id\": \"s2\",\n\"tool_name\": \"Read\"\n}";
    let cursor = Cursor::new(json.as_bytes());
    let input = read_hook_input_from(cursor).expect("should parse multiline");
    assert_eq!(input.session_id, "s2");
    assert_eq!(input.tool_name, "Read");
}
