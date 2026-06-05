//! Tests for enforce module.

use super::{EnforcementDecision, check_enforcement, format_block_reason};
use crate::file_matcher::FileMatchResult;

fn make_result(critical: &[&str], advisory: &[&str]) -> FileMatchResult {
    FileMatchResult {
        critical: critical.iter().map(ToString::to_string).collect(),
        advisory: advisory.iter().map(ToString::to_string).collect(),
    }
}

fn invoked(skills: &[&str]) -> Vec<String> {
    skills.iter().map(ToString::to_string).collect()
}

#[test]
fn test_allowed_when_no_matches() {
    let m = make_result(&[], &[]);
    let dec = check_enforcement(&m, &invoked(&["rust", "security"]));
    assert!(matches!(dec, EnforcementDecision::Allowed));
}

#[test]
fn test_blocked_missing_critical() {
    let m = make_result(&["security"], &[]);
    let dec = check_enforcement(&m, &invoked(&["rust"]));
    match dec {
        EnforcementDecision::Blocked {
            missing_critical, ..
        } => {
            assert_eq!(missing_critical, vec!["security"]);
        }
        EnforcementDecision::Allowed => panic!("expected Blocked"),
    }
}

#[test]
fn test_allowed_critical_satisfied() {
    let m = make_result(&["security"], &["rust"]);
    let dec = check_enforcement(&m, &invoked(&["security"]));
    assert!(matches!(dec, EnforcementDecision::Allowed));
}

#[test]
fn test_blocked_advisory_none_invoked() {
    let m = make_result(&[], &["rust", "coding-guidelines"]);
    let dec = check_enforcement(&m, &invoked(&[]));
    assert!(matches!(dec, EnforcementDecision::Blocked { .. }));
}

#[test]
fn test_allowed_advisory_one_invoked() {
    let m = make_result(&[], &["rust", "coding-guidelines"]);
    let dec = check_enforcement(&m, &invoked(&["rust"]));
    assert!(matches!(dec, EnforcementDecision::Allowed));
}

#[test]
fn test_format_block_reason_critical() {
    let m = make_result(&["security"], &[]);
    let dec = check_enforcement(&m, &invoked(&[]));
    let msg = format_block_reason(&dec, "src/auth/login.rs");
    assert!(msg.contains("SKILL VIOLATION"));
    assert!(msg.contains("security"));
}

#[test]
fn test_format_block_reason_advisory() {
    let m = make_result(&[], &["rust", "coding-guidelines"]);
    let dec = check_enforcement(&m, &invoked(&[]));
    let msg = format_block_reason(&dec, "src/main.rs");
    assert!(msg.contains("SKILL VIOLATION"));
    assert!(msg.contains("Invoke at least one"));
}
