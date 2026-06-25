use crate::Vendor;

#[test]
fn output_sink_defaults_to_claude_code_then_tracks_set_vendor() {
    assert_eq!(
        crate::output_vendor(),
        Vendor::ClaudeCode,
        "unset => canonical default"
    );
    crate::set_output_context(Vendor::Cursor, "Stop");
    assert_eq!(crate::output_vendor(), Vendor::Cursor);
    assert_eq!(
        crate::output_event(),
        "Stop",
        "the answered event is recorded too"
    );
    crate::set_output_context(Vendor::ClaudeCode, "");
}

#[test]
fn cursor_armed_sink_never_emits_a_top_level_null_pair() {
    let json = Vendor::Cursor.render(&crate::HookResponse::new_approve("ok"));
    assert!(
        !json.contains(r#""continue":null"#),
        "no null continue: {json}"
    );
    assert!(
        !json.contains(r#""permission":null"#),
        "no null permission: {json}"
    );
}
