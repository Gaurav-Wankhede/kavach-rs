use super::is_scannable_rust;

#[test]
fn excludes_test_file_variants() {
    assert!(is_scannable_rust("crates/x/src/cmd/audit/walk.rs"));
    assert!(!is_scannable_rust("crates/x/src/foo_test.rs"));
    assert!(!is_scannable_rust("crates/x/src/foo_tests.rs"));
    assert!(!is_scannable_rust("crates/x/src/tests.rs"));
    assert!(!is_scannable_rust("crates/x/tests/integration.rs"));
    assert!(!is_scannable_rust("crates/x/src/foo_test_helpers.rs"));
    assert!(is_scannable_rust("crates/x/src/latest.rs"));
    assert!(!is_scannable_rust("crates/x/src/foo.toml"));
}
