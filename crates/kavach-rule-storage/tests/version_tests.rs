//! Tests for version tracking and hash computation.

use kavach_rule_storage::RuleVersion;

#[test]
fn compute_hash_deterministic() {
    let h1 = RuleVersion::compute_hash("hello world");
    let h2 = RuleVersion::compute_hash("hello world");
    assert_eq!(h1, h2);
}

#[test]
fn compute_hash_differs_for_different_content() {
    let h1 = RuleVersion::compute_hash("alpha");
    let h2 = RuleVersion::compute_hash("beta");
    assert_ne!(h1, h2);
}

#[test]
fn compute_hash_is_hex_blake3() {
    let h = RuleVersion::compute_hash("test");
    assert_eq!(h.len(), 64, "BLAKE3 hex digest must be 64 chars (32 bytes)");
    assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn next_version_unchanged() {
    let v = RuleVersion::next_version(3, "abc", "abc");
    assert_eq!(v, 3);
}

#[test]
fn next_version_increments_on_change() {
    let v = RuleVersion::next_version(3, "abc", "def");
    assert_eq!(v, 4);
}
