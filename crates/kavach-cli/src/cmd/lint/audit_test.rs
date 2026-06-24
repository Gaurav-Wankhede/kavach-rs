use super::{is_test_path, tag_line};

#[test]
fn dead_code_allow_is_delete_tagged() {
    let (tag, _what, weight) = tag_line("#[allow(dead_code)]").unwrap();
    assert_eq!(tag, "delete");
    assert!(weight >= 5);
}

#[test]
fn identity_map_is_shrink() {
    let (tag, _w, _wt) = tag_line("    let y = xs.iter().map(|x| x).collect();").unwrap();
    assert_eq!(tag, "shrink");
}

#[test]
fn unwrap_or_else_vec_new_is_stdlib() {
    let (tag, _w, _wt) = tag_line("    let v = opt.unwrap_or_else(|| Vec::new());").unwrap();
    assert_eq!(tag, "stdlib");
}

#[test]
fn clean_line_is_none() {
    assert!(tag_line("    let total = a + b;").is_none());
}

#[test]
fn test_paths_are_skipped() {
    assert!(is_test_path("crates/x/src/foo_test.rs"));
    assert!(is_test_path("crates/x/tests/integration.rs"));
    assert!(!is_test_path("crates/x/src/foo.rs"));
}
