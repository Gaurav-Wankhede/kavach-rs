//! Pub-export counting, mod/lib/main + index skips, JS export detection, and
//! empty / non-code pass-through.
use super::check_orphan_risk;
use super::js::check_js_orphan;
use super::rust::{check_rust_orphan, count_pub_exports};

#[test]
fn test_count_pub_exports() {
    let content = "pub fn foo() {}\npub struct Bar;\nfn private() {}";
    assert_eq!(count_pub_exports(content), 2);
}

#[test]
fn test_count_pub_exports_private_fns() {
    let content = "fn private() {}\nfn also_private() {}";
    assert_eq!(count_pub_exports(content), 0);
}

#[test]
fn test_skip_mod_lib_main() {
    assert!(check_rust_orphan("/src/mod.rs", "pub fn x() {}").is_none());
    assert!(check_rust_orphan("/src/lib.rs", "pub fn x() {}").is_none());
    assert!(check_rust_orphan("/src/main.rs", "pub fn x() {}").is_none());
}

#[test]
fn test_js_export_detection() {
    let content = "export function Foo() {}\nexport const BAR = 1;";
    let result = check_js_orphan("/src/Foo.tsx", content);
    assert!(result.is_some());
    assert!(result.as_ref().is_some_and(|r| r.contains("2 export(s)")));
}

#[test]
fn test_js_index_skipped() {
    assert!(check_js_orphan("/src/index.ts", "export const x = 1;").is_none());
}

#[test]
fn test_no_orphan_for_empty() {
    assert!(check_orphan_risk("", "").is_none());
    assert!(check_orphan_risk("foo.rs", "").is_none());
}

#[test]
fn test_non_code_files_skipped() {
    assert!(check_orphan_risk("README.md", "# Hello").is_none());
    assert!(check_orphan_risk("data.json", "{}").is_none());
}
