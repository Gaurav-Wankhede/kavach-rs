//! `is_git_add_all` tests: blanket-staging detection vs specific-path allow.
use super::super::is_git_add_all;

#[test]
fn test_git_add_all_detected() {
    assert!(is_git_add_all("git add ."));
    assert!(is_git_add_all("git add -A"));
    assert!(is_git_add_all("git add --all"));
}

#[test]
fn test_git_add_specific_allowed() {
    assert!(!is_git_add_all("git add src/main.rs"));
    assert!(!is_git_add_all("git add Backend/Cargo.toml"));
    assert!(!is_git_add_all("git commit -m 'fix'"));
    assert!(!is_git_add_all("cargo test"));
}
