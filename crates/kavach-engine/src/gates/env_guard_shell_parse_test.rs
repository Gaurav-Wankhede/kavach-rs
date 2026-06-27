//! Segment first-word matching (incl. argv-flag rejection), redirect skipping,
//! and command-position detection at start / after-separator / inside-flag.
use super::{first_word_is, first_word_matches, is_command_position, skip_shell_redirects};

#[test]
fn first_word_matches_at_segment_starts() {
    assert!(first_word_matches("source .env && cargo run", &["source"]));
    assert!(first_word_matches("cd /tmp; source .env", &["source"]));
    assert!(first_word_matches("a | b | source .env", &["source"]));
}

#[test]
fn first_word_skips_argv_flags() {
    // --source is an argv flag, not a builtin in command position.
    assert!(!first_word_matches(
        "sqlx --source migrations_local migrate run",
        &["source"]
    ));
}

#[test]
fn first_word_is_single_name_wrapper() {
    assert!(first_word_is("cargo build", "cargo"));
    assert!(!first_word_is("npm cargo", "cargo"));
}

#[test]
fn skip_redirect_2_to_devnull() {
    assert_eq!(skip_shell_redirects("2>/dev/null cargo run"), "cargo run");
}

#[test]
fn skip_redirect_combined_2_to_1() {
    assert_eq!(
        skip_shell_redirects(">/dev/null 2>&1 cargo run"),
        "cargo run"
    );
}

#[test]
fn skip_redirect_stops_at_non_redirect() {
    assert_eq!(skip_shell_redirects("cargo run"), "cargo run");
}

#[test]
fn command_position_at_start() {
    assert!(is_command_position(b"source .env", 0));
}

#[test]
fn command_position_after_separator() {
    let cmd = b"cd /tmp && source .env";
    // Position of "source" is after "&& "; check it.
    let pos = cmd.iter().position(|&b| b == b's').unwrap_or(0);
    assert!(is_command_position(cmd, pos));
}

#[test]
fn command_position_rejects_argv_flag() {
    let cmd = b"sqlx --source migrations";
    // Position of "source" inside --source is NOT command position.
    let pos = cmd.windows(6).position(|w| w == b"source").unwrap_or(0);
    assert!(!is_command_position(cmd, pos));
}
