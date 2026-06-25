use crate::Vendor;

#[test]
fn detects_cursor_from_its_signature_fields() {
    let p = r#"{"conversation_id":"c1","workspace_roots":["/r"],"prompt":"x"}"#;
    assert_eq!(Vendor::detect(p), Vendor::Cursor);
}

#[test]
fn detects_codex_from_turn_id() {
    let p = r#"{"session_id":"s1","turn_id":"t1","hook_event_name":"PreToolUse"}"#;
    assert_eq!(Vendor::detect(p), Vendor::Codex);
}

#[test]
fn detects_cursor_from_camelcase_event_when_id_fields_absent() {
    let p = r#"{"hook_event_name":"workspaceOpen","workspace_roots":["/r"]}"#;
    assert_eq!(Vendor::detect(p), Vendor::Cursor);
    let bare = r#"{"hook_event_name":"beforeSubmitPrompt","prompt":"hi"}"#;
    assert_eq!(Vendor::detect(bare), Vendor::Cursor);
}

#[test]
fn detects_cursor_from_cursor_version_field() {
    let p = r#"{"cursor_version":"1.2.3","hook_event_name":"PreToolUse"}"#;
    assert_eq!(Vendor::detect(p), Vendor::Cursor);
}

#[test]
fn pascalcase_event_is_not_mistaken_for_cursor() {
    let p = r#"{"hook_event_name":"PreToolUse","tool_name":"Bash"}"#;
    assert_eq!(Vendor::detect_from_payload(p), None);
}

#[test]
fn unknown_or_plain_payload_defaults_to_claude_code() {
    assert_eq!(
        Vendor::detect_from_payload(r#"{"session_id":"s1","tool_name":"Bash"}"#),
        None
    );
    assert_eq!(
        Vendor::detect_from_payload("not json"),
        None,
        "unparseable => no payload signal => CC default"
    );
}

#[test]
fn an_explicit_flag_overrides_the_payload_sniff() {
    let cursor_shaped = r#"{"conversation_id":"c1"}"#;
    assert_eq!(Vendor::resolve(Some("codex"), cursor_shaped), Vendor::Codex);
    assert_eq!(Vendor::resolve(Some("cursor"), "{}"), Vendor::Cursor);
}

#[test]
fn an_unknown_flag_falls_through_to_detect() {
    let p = r#"{"conversation_id":"c1"}"#;
    assert_eq!(
        Vendor::resolve(Some("nonsense"), p),
        Vendor::Cursor,
        "bad flag => sniff"
    );
}
