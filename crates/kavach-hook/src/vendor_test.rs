//! Native-edge proofs: vendor detection (hybrid), input lowering per harness, and
//! native output rendering incl. each vendor's failure policy.

use super::{Vendor, cursor};
use kavach_types::HookResponse;

// --- detection (hybrid: flag > env > sniff > default) ---

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
fn unknown_or_plain_payload_defaults_to_claude_code() {
    assert_eq!(Vendor::detect(r#"{"session_id":"s1","tool_name":"Bash"}"#), Vendor::ClaudeCode);
    assert_eq!(Vendor::detect("not json"), Vendor::ClaudeCode, "unparseable => safe default");
}

#[test]
fn an_explicit_flag_overrides_the_payload_sniff() {
    // Payload looks like Cursor, but the flag forces Codex (hybrid: flag wins).
    let cursor_shaped = r#"{"conversation_id":"c1"}"#;
    assert_eq!(Vendor::resolve(Some("codex"), cursor_shaped), Vendor::Codex);
    assert_eq!(Vendor::resolve(Some("cursor"), "{}"), Vendor::Cursor);
}

#[test]
fn an_unknown_flag_falls_through_to_detect() {
    let p = r#"{"conversation_id":"c1"}"#;
    assert_eq!(Vendor::resolve(Some("nonsense"), p), Vendor::Cursor, "bad flag => sniff");
}

// --- Cursor native input lowering ---

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
    assert_eq!(input.hook_event_name, "PreToolUse", "beforeShellExecution -> PreToolUse");
}

#[test]
fn cursor_input_tolerates_nulls_and_missing_fields() {
    let p = r#"{"conversation_id":null,"prompt":"hi","workspace_roots":null,"metadata":null}"#;
    let input = cursor::lower(p).expect("nulls must not block");
    assert_eq!(input.prompt, "hi");
    assert_eq!(input.session_id, "");
    assert_eq!(input.cwd, "");
}

// --- Codex native input lowering (CC-compatible) ---

#[test]
fn codex_input_is_claude_code_compatible_with_extras_ignored() {
    let p = r#"{"session_id":"s1","turn_id":"t1","permission_mode":"plan",
                "tool_name":"Write","hook_event_name":"PreToolUse"}"#;
    let input = super::codex::lower(p).expect("codex lowers");
    assert_eq!(input.session_id, "s1");
    assert_eq!(input.tool_name, "Write", "CC field names pass through");
}

// --- native output rendering + failure policy ---

#[test]
fn cursor_block_renders_the_native_deny_contract() {
    let json = cursor::render(&HookResponse::new_block("nope"));
    assert!(json.contains(r#""continue":false"#), "got {json}");
    assert!(json.contains(r#""permission":"deny""#), "got {json}");
    assert!(json.contains("nope"), "reason carried as user/agent message: {json}");
    assert!(!json.contains(r#""decision""#), "must NOT emit Claude-Code shape");
}

#[test]
fn cursor_approve_renders_allow() {
    let json = cursor::render(&HookResponse::new_approve("ok"));
    assert!(json.contains(r#""continue":true"#), "got {json}");
    assert!(json.contains(r#""permission":"allow""#), "got {json}");
}

#[test]
fn cursor_fails_open_on_error() {
    let json = cursor::fail_open();
    assert!(json.contains(r#""continue":true"#), "Cursor's native default is allow");
    assert!(json.contains(r#""permission":"allow""#));
}

#[test]
fn codex_blocks_via_exit_code_two_not_the_body() {
    assert_eq!(Vendor::Codex.block_exit_code(), 2, "Codex hard-block = exit 2");
    assert_eq!(Vendor::ClaudeCode.block_exit_code(), 0);
    assert_eq!(Vendor::Cursor.block_exit_code(), 0);
}

#[test]
fn claude_code_render_is_the_canonical_json_unchanged() {
    let json = Vendor::ClaudeCode.render(&HookResponse::new_block("x"));
    assert!(json.contains(r#""decision":"block""#), "CC keeps canonical shape: {json}");
}

// --- thread-local output sink (the happy-path native translation) ---

#[test]
fn output_sink_defaults_to_claude_code_then_tracks_set_vendor() {
    // The sink is what makes a gate's SELF-EMITTED verdict native: the edge arms
    // it once, every `output(&resp)` then renders in that dialect. Proven here on
    // the selector; the render mapping itself is covered above.
    assert_eq!(crate::output_vendor(), Vendor::ClaudeCode, "unset => canonical default");
    crate::set_output_vendor(Vendor::Cursor);
    assert_eq!(crate::output_vendor(), Vendor::Cursor);
    // Restore so we don't leak the dialect into sibling tests on this thread.
    crate::set_output_vendor(Vendor::ClaudeCode);
}

#[test]
fn cursor_armed_sink_never_emits_a_top_level_null_pair() {
    // The original Cursor wedge: an allow rendered in CC's shape, so Cursor read
    // its absent `continue`/`permission` as null and `invalid type: null` blocked
    // the IDE. With the sink armed, the rendered body carries real booleans.
    let json = Vendor::Cursor.render(&HookResponse::new_approve("ok"));
    assert!(!json.contains(r#""continue":null"#), "no null continue: {json}");
    assert!(!json.contains(r#""permission":null"#), "no null permission: {json}");
}
