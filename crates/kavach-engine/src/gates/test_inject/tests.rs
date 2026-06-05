//! Coverage: no-debt, crate scoping (single/multi), workspace fallback, path parse.
use kavach_session::SessionState;

use super::build_test_context;
use super::path::crate_name_from_path;

#[test]
fn test_no_pending() {
    let mut s = SessionState::default();
    assert!(build_test_context(&mut s).is_none());
}

#[test]
fn test_with_pending_scopes_to_crate() {
    let mut s = SessionState::default();
    s.test_files_pending.push(
        "/Users/gauravwankhede/kavach-rs/crates/kavach-engine/src/gates/rag_router.rs".into(),
    );
    let ctx = build_test_context(&mut s).expect("should produce context");
    assert!(ctx.contains("TEST_ENFORCEMENT"));
    assert!(
        ctx.contains("-p kavach-engine"),
        "action must be scoped: {ctx}"
    );
    assert!(
        !ctx.contains("--workspace"),
        "must not run full workspace: {ctx}"
    );
}

#[test]
fn test_with_two_crates_emits_both_flags() {
    let mut s = SessionState::default();
    s.test_files_pending
        .push("/home/user/proj/crates/kavach-engine/src/gates/foo.rs".into());
    s.test_files_pending
        .push("/home/user/proj/crates/kavach-db/src/memory.rs".into());
    let ctx = build_test_context(&mut s).expect("context");
    assert!(
        ctx.contains("-p kavach-db"),
        "missing kavach-db flag: {ctx}"
    );
    assert!(
        ctx.contains("-p kavach-engine"),
        "missing kavach-engine flag: {ctx}"
    );
}

#[test]
fn test_workspace_fallback_for_non_crate_paths() {
    let mut s = SessionState::default();
    s.test_files_pending.push("src/main.rs".into());
    let ctx = build_test_context(&mut s).expect("context");
    assert!(
        ctx.contains("--workspace"),
        "non-crate path must fall back to workspace: {ctx}"
    );
}

#[test]
fn test_crate_name_from_path_extracts_name() {
    assert_eq!(
        crate_name_from_path("/Users/x/kavach-rs/crates/kavach-engine/src/gates/foo.rs"),
        Some("kavach-engine".into())
    );
    assert_eq!(crate_name_from_path("src/foo.rs"), None);
    assert_eq!(crate_name_from_path("/crates//src/foo.rs"), None);
}
