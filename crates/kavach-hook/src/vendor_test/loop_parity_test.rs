use crate::Vendor;
use kavach_types::HookResponse;

fn stop_continue() -> HookResponse {
    let mut resp = HookResponse::new_block("[AUTO_CONTINUE] Kanban has pending work — do not stop.");
    resp.hook_specific_output = Some(kavach_types::HookSpecificOutput {
        hook_event_name: "Stop".to_owned(),
        ..Default::default()
    });
    resp
}

#[test]
fn auto_continue_reaches_claude_code_stop() {
    let json = Vendor::ClaudeCode.render_for(&stop_continue(), "Stop");
    assert!(json.contains(r#""decision":"block""#), "cc block: {json}");
    assert!(json.contains("AUTO_CONTINUE"), "cc carries continuation: {json}");
}

#[test]
fn auto_continue_reaches_cursor_stop_as_followup_message() {
    let json = Vendor::Cursor.render_for(&stop_continue(), "Stop");
    assert!(json.contains("followup_message"), "cursor stop key: {json}");
    assert!(json.contains("AUTO_CONTINUE"), "cursor carries continuation: {json}");
    assert!(!json.contains(r#""continue""#), "cursor stop has no `continue` field: {json}");
}

#[test]
fn auto_continue_reaches_codex_stop_body() {
    let json = Vendor::Codex.render_for(&stop_continue(), "Stop");
    assert!(json.contains(r#""decision":"block""#), "codex block: {json}");
    assert!(json.contains("AUTO_CONTINUE"), "codex carries continuation: {json}");
}

#[test]
fn auto_continue_reaches_pi_agent_end_as_block() {
    let json = Vendor::Pi.render_for(&stop_continue(), "Stop");
    assert!(json.contains(r#""block":true"#), "pi block: {json}");
    assert!(json.contains("AUTO_CONTINUE"), "pi carries continuation: {json}");
}

#[test]
fn clean_stop_does_not_resubmit_on_cursor() {
    let allow = HookResponse::new_approve("");
    let json = Vendor::Cursor.render_for(&allow, "Stop");
    assert!(!json.contains("followup_message"), "clean stop must not resubmit: {json}");
}
