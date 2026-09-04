//! Tests for RTK and toolbelt command rewriters.
use super::check_rewrite;

#[test]
fn test_rtk_already_prefixed_returns_none() {
    assert!(check_rewrite("rtk cargo test").is_none());
    assert!(check_rewrite("rtk").is_none());
    assert!(check_rewrite("").is_none());
}

#[test]
fn test_env_rewrites_to_rtk_env() {
    assert_eq!(check_rewrite("env").as_deref(), Some("rtk env"));
    assert_eq!(check_rewrite("printenv").as_deref(), Some("rtk env"));
    assert_eq!(
        check_rewrite("printenv USER").as_deref(),
        Some("rtk env -f USER")
    );
}

#[test]
fn test_cargo_subcommands_rewrite() {
    assert_eq!(
        check_rewrite("cargo test").as_deref(),
        Some("rtk cargo test")
    );
    assert_eq!(
        check_rewrite("cargo check").as_deref(),
        Some("rtk cargo check")
    );
    assert_eq!(
        check_rewrite("cargo clippy --all").as_deref(),
        Some("rtk cargo clippy --all")
    );
}

#[test]
fn test_git_subcommands_rewrite() {
    assert_eq!(
        check_rewrite("git status").as_deref(),
        Some("rtk git status")
    );
    assert_eq!(
        check_rewrite("git diff HEAD~1").as_deref(),
        Some("rtk git diff HEAD~1")
    );
}

#[test]
fn test_toolbelt_fallback_for_cat_and_jq() {
    assert_eq!(
        check_rewrite("cat Cargo.toml").as_deref(),
        Some("bat --paging=never Cargo.toml")
    );
    assert_eq!(
        check_rewrite("jq '.version' package.json").as_deref(),
        Some("jaq '.version' package.json")
    );
}

#[test]
fn test_pipeline_rewrites_compound_commands() {
    assert_eq!(
        check_rewrite("cargo check && git status").as_deref(),
        Some("rtk cargo check && rtk git status")
    );
    assert_eq!(
        check_rewrite("cat Cargo.toml | grep version").as_deref(),
        Some("bat --paging=never Cargo.toml | rtk grep version")
    );
}
