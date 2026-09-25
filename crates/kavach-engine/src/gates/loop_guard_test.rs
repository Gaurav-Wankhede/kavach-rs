//! Window threshold, history-bound, and entry-parse/normalize coverage.
use super::detect::{WINDOW_TURNS, check_bash_loop, check_bash_loop_deny};
use super::history::{HISTORY_SIZE, normalize_command, parse_entry, record_command, truncate};
use kavach_session::SessionState;

fn make_session(turn: i32) -> SessionState {
    let mut s = SessionState::default();
    s.turn_count = turn;
    s
}

#[test]
fn normalize_whitespace() {
    assert_eq!(
        normalize_command("cargo  build  --release"),
        "cargo build --release"
    );
}

#[test]
fn truncate_short() {
    assert_eq!(truncate("hello", 10), "hello");
}

#[test]
fn truncate_long() {
    assert_eq!(truncate("hello world", 5), "hello");
}

#[test]
fn parse_entry_with_turn() {
    assert_eq!(parse_entry("5:cargo check"), (5, "cargo check"));
}

#[test]
fn parse_entry_legacy_no_turn() {
    assert_eq!(parse_entry("cargo check"), (0, "cargo check"));
}

#[test]
fn no_block_before_threshold() {
    let mut session = make_session(5);
    record_command(&mut session, "cargo check");
    record_command(&mut session, "cargo check");
    assert!(check_bash_loop(&session, "cargo check").is_none());
}

#[test]
fn blocks_at_threshold_within_window() {
    let mut session = make_session(5);
    record_command(&mut session, "cargo check");
    record_command(&mut session, "cargo check");
    record_command(&mut session, "cargo check");
    assert!(check_bash_loop(&session, "cargo check").is_some());
}

#[test]
fn no_block_when_old_entries_outside_window() {
    let mut session = make_session(1);
    record_command(&mut session, "cargo check");
    record_command(&mut session, "cargo check");
    // Advance turn beyond the window; the 2 old entries age out.
    session.turn_count = 1 + WINDOW_TURNS + 1;
    record_command(&mut session, "cargo check");
    // Only 1 entry inside the window → no block.
    assert!(check_bash_loop(&session, "cargo check").is_none());
}

#[test]
fn record_command_bounds_history() {
    let mut session = make_session(0);
    for i in 0..HISTORY_SIZE + 5 {
        session.turn_count = i32::try_from(i).unwrap_or(i32::MAX);
        record_command(&mut session, &format!("cmd{i}"));
    }
    assert!(session.recent_commands.len() <= HISTORY_SIZE);
}

/// FP-bound suite for the tight-loop DENY (P0 bar): only 5+ consecutive
/// byte-identical (post-normalization) commands deny. Everything else passes.
fn repeat(session: &mut SessionState, cmd: &str, n: usize) {
    for _ in 0..n {
        record_command(session, cmd);
    }
}

#[test]
fn deny_fires_on_fifth_consecutive() {
    // 4 recorded + current = 5 in a row -> deny (the reported 15x spam shape).
    let mut session = make_session(5);
    repeat(&mut session, "rtk rg -n foo src/", 4);
    let reason = check_bash_loop_deny(&session, "rtk rg -n foo src/");
    assert!(reason.is_some(), "5th consecutive identical call must deny");
    assert!(reason.unwrap().contains("LOOP_DENY"));
}

#[test]
fn four_consecutive_passes() {
    let mut session = make_session(5);
    repeat(&mut session, "cargo check", 3);
    assert!(check_bash_loop_deny(&session, "cargo check").is_none());
}

#[test]
fn interleaved_command_breaks_the_run() {
    // Same command 6x total but never 5 consecutively -> no deny.
    let mut session = make_session(5);
    repeat(&mut session, "cargo check", 2);
    record_command(&mut session, "cargo build");
    repeat(&mut session, "cargo check", 3);
    assert!(check_bash_loop_deny(&session, "cargo check").is_none());
}

#[test]
fn whitespace_variants_count_as_identical() {
    // Normalization collapses whitespace: padding evasion still denies.
    let mut session = make_session(5);
    repeat(&mut session, "cargo   check", 4);
    assert!(check_bash_loop_deny(&session, "cargo check").is_some());
}

#[test]
fn different_command_passes() {
    let mut session = make_session(5);
    repeat(&mut session, "cargo check", 4);
    assert!(check_bash_loop_deny(&session, "cargo build").is_none());
}

#[test]
fn empty_command_never_denies() {
    let mut session = make_session(5);
    repeat(&mut session, "", 4);
    assert!(check_bash_loop_deny(&session, "").is_none());
    assert!(check_bash_loop_deny(&session, "   ").is_none());
}

#[test]
fn bypass_env_opts_out() {
    let mut session = make_session(5);
    repeat(&mut session, "cargo check", 4);
    temp_env::with_var("KAVACH_LOOP_BYPASS", Some("1"), || {
        assert!(check_bash_loop_deny(&session, "cargo check").is_none());
    });
    assert!(check_bash_loop_deny(&session, "cargo check").is_some());
}
