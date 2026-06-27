//! Path-gate classification: universal-doc allowlist + forbidden six-file
//! markdown patterns (context/docs dirs, uppercase canon names, numbered specs).
use super::classify::{is_allowlisted, is_forbidden};

#[test]
fn test_allowlist_claude_md() {
    assert!(is_allowlisted("CLAUDE.md"));
    assert!(is_allowlisted("/path/to/CLAUDE.md"));
}

#[test]
fn test_allowlist_readme() {
    assert!(is_allowlisted("README.md"));
}

#[test]
fn test_forbidden_context_path() {
    assert!(is_forbidden("context/project-overview.md"));
    assert!(is_forbidden("docs/architecture.md"));
}

#[test]
fn test_forbidden_uppercase() {
    assert!(is_forbidden("PROJECT-OVERVIEW.md"));
    assert!(is_forbidden("/path/ARCHITECTURE.md"));
}

#[test]
fn test_forbidden_spec_number() {
    assert!(is_forbidden("specs/123-feature.md"));
    assert!(is_forbidden("spec/42_auth.md"));
}

#[test]
fn test_forbidden_spec_md() {
    assert!(is_forbidden("something.spec.md"));
}

#[test]
fn test_allowed_regular_rs() {
    assert!(!is_forbidden("src/main.rs"));
}
