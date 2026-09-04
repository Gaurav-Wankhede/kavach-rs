//! Tests for RTK command rewriter.
use super::check_rewrite;

#[test]
fn test_rtk_already_prefixed_returns_none() {
    assert!(check_rewrite("rtk cargo test").is_none());
    assert!(check_rewrite("rtk").is_none());
}

#[test]
fn test_env_rewrites_to_rtk_env() {
    if std::path::Path::new("/opt/homebrew/bin/rtk").exists() {
        assert_eq!(check_rewrite("env").as_deref(), Some("rtk env"));
        assert_eq!(check_rewrite("printenv").as_deref(), Some("rtk env"));
        assert_eq!(
            check_rewrite("printenv USER").as_deref(),
            Some("rtk env -f USER")
        );
    }
}

#[test]
fn test_cargo_test_rewrites() {
    if std::path::Path::new("/opt/homebrew/bin/rtk").exists() {
        assert_eq!(
            check_rewrite("cargo test").as_deref(),
            Some("rtk cargo test")
        );
    }
}
