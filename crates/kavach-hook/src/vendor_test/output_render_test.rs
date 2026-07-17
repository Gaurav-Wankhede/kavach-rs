use crate::{Vendor, vendor::cursor};
use kavach_types::HookResponse;

#[test]
fn cursor_block_renders_the_native_deny_contract() {
    let json = cursor::render(&HookResponse::new_block("nope"), "PreToolUse");
    assert!(
        !json.contains(r#""continue""#),
        "pre-tool has no continue: {json}"
    );
    assert!(json.contains(r#""permission":"deny""#), "got {json}");
    assert!(
        json.contains("nope"),
        "reason carried as user/agent message: {json}"
    );
    assert!(
        !json.contains(r#""decision""#),
        "must NOT emit Claude-Code shape"
    );
}

#[test]
fn cursor_approve_renders_allow() {
    let json = cursor::render(&HookResponse::new_approve("ok"), "PreToolUse");
    assert!(
        !json.contains(r#""continue""#),
        "pre-tool has no continue: {json}"
    );
    assert!(json.contains(r#""permission":"allow""#), "got {json}");
}

#[test]
fn cursor_fails_open_on_error() {
    let json = cursor::fail_open();
    assert!(
        json.contains(r#""continue":true"#),
        "Cursor's native default is allow"
    );
    assert!(json.contains(r#""permission":"allow""#));
}

#[test]
fn codex_blocks_via_exit_code_two_not_the_body() {
    assert_eq!(
        Vendor::Codex.block_exit_code(),
        2,
        "Codex hard-block = exit 2"
    );
    assert_eq!(Vendor::ClaudeCode.block_exit_code(), 0);
    assert_eq!(Vendor::Cursor.block_exit_code(), 0);
}

#[test]
fn kimi_blocks_via_exit_code_two_and_renders_canonical_json() {
    assert_eq!(Vendor::Kimi.block_exit_code(), 2, "Kimi hard-block = exit 2");
    let json = Vendor::Kimi.render(&HookResponse::new_block("x"));
    assert!(
        json.contains(r#""decision":"block""#),
        "Kimi keeps the canonical CC shape: {json}"
    );
}

#[test]
fn claude_code_render_is_the_canonical_json_unchanged() {
    let json = Vendor::ClaudeCode.render(&HookResponse::new_block("x"));
    assert!(
        json.contains(r#""decision":"block""#),
        "CC keeps canonical shape: {json}"
    );
}
