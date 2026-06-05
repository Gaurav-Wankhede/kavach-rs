//! Test-scope extraction + active-crate clear tests.
use crate::gates::post_tool_bash::tests_track::{clear_test_run, extract_test_scope};

#[test]
fn test_scope_extraction_cargo_p() {
    let s = extract_test_scope("cargo test -p my-service --lib");
    assert_eq!(s, vec!["my-service"]);
}

#[test]
fn test_scope_extraction_bun_path() {
    let s = extract_test_scope("bun test packages/shared/src/lib/hooks.test.ts");
    assert!(s.iter().any(|p| p.contains("hooks.test.ts")));
}

#[test]
fn test_scope_extraction_workspace_empty() {
    let s = extract_test_scope("cargo test --workspace");
    assert!(s.is_empty());
}

#[test]
fn should_clear_active_test_crate_on_completion() {
    let mut session = kavach_session::SessionState::default();
    session.active_test_crates.push("kavach-engine".into());
    clear_test_run(&mut session, "cargo test -p kavach-engine --lib");
    assert!(session.active_test_crates.is_empty());
}

#[test]
fn should_not_clear_unrelated_crate() {
    let mut session = kavach_session::SessionState::default();
    session.active_test_crates.push("kavach-db".into());
    clear_test_run(&mut session, "cargo test -p kavach-engine --lib");
    assert_eq!(session.active_test_crates.len(), 1);
}
