//! Test `production_stem_of` with the real nested path that failed.
use crate::gates::pre_write_guards::tdd_guard;

#[test]
fn production_stem_of_nested_path_selection_test() {
    let nested_path = "crates/kavach-cli/src/cmd/audit/lens/selection_test.rs";
    let stem = tdd_guard::production_stem_of(nested_path);
    assert_eq!(
        stem,
        Some("selection".to_owned()),
        "nested path {nested_path} should map to stem 'selection', got {stem:?}"
    );
}

#[test]
fn is_real_verify_with_path_filter() {
    let cmd = "cargo nextest run -p kavach-cli audit::lens::selection_test";
    let is_verify = kavach_patterns::reward::is_real_verify(cmd);
    assert!(
        is_verify,
        "command with path filter {} should be real_verify, got {}",
        cmd,
        is_verify
    );
}
