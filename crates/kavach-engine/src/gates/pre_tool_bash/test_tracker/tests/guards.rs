//! `check_unscoped_test_run` / `check_duplicate_test_run` gate tests.
use crate::gates::pre_tool_bash::test_tracker::guards::{
    check_duplicate_test_run, check_unscoped_test_run,
};

#[test]
fn should_block_duplicate_test_run() {
    let mut session = kavach_session::SessionState::default();
    session.active_test_crates.push("kavach-engine".into());
    assert!(check_duplicate_test_run(&session, "cargo test -p kavach-engine --lib").is_some());
}

#[test]
fn should_allow_test_run_when_no_active_run() {
    let session = kavach_session::SessionState::default();
    assert!(check_duplicate_test_run(&session, "cargo test -p kavach-engine --lib").is_none());
}

#[test]
fn should_allow_different_crate_test_run() {
    let mut session = kavach_session::SessionState::default();
    session.active_test_crates.push("kavach-db".into());
    assert!(check_duplicate_test_run(&session, "cargo test -p kavach-engine --lib").is_none());
}

#[test]
fn should_block_unscoped_cargo_test() {
    assert!(check_unscoped_test_run("cargo test 2>&1 | tail -20").is_some());
    assert!(check_unscoped_test_run("cargo nextest run --no-fail-fast").is_some());
}

#[test]
fn should_not_classify_quoted_pipe_as_unscoped() {
    assert!(check_unscoped_test_run(r"rg -n 'x|cargo nextest' src/").is_none());
}

#[test]
fn should_allow_scoped_cargo_test() {
    assert!(check_unscoped_test_run("cargo test -p kavach-engine").is_none());
    assert!(check_unscoped_test_run("cargo nextest run -p kavach-db").is_none());
}

#[test]
fn should_allow_explicit_workspace_flag() {
    assert!(check_unscoped_test_run("cargo nextest run --workspace").is_none());
}

#[test]
fn should_auto_expire_stale_test_tracking() {
    let mut session = kavach_session::SessionState::default();
    session.active_test_crates.push("kavach-engine".into());
    session.turn_count = 20;
    session.last_write_turn = 10; // 10 turns ago — stale
    assert!(check_duplicate_test_run(&session, "cargo test -p kavach-engine").is_none());
}
