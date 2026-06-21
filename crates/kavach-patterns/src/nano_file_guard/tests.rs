use super::*;

#[path = "inline_test_rule_tests.rs"]
mod inline_test_rule;

#[test]
fn mod_rs_blocked() {
    let v = detect("crates/foo/src/bar/mod.rs", "", "Write");
    assert!(
        v.iter()
            .any(|x| x.severity == NanoSeverity::P0Block && x.pattern == "legacy mod.rs file")
    );
}

#[test]
fn depth_at_limit_allowed() {
    let path = "crates/foo/src/a/b/c/d/e/f/g/leaf.rs";
    let v = detect(path, "fn x() {}\n", "Write");
    assert!(!v.iter().any(|x| x.pattern.starts_with("directory depth")));
}

#[test]
fn depth_over_limit_blocked() {
    let path = "crates/foo/src/a/b/c/d/e/f/g/h/leaf.rs";
    let v = detect(path, "fn x() {}\n", "Write");
    assert!(
        v.iter()
            .any(|x| x.severity == NanoSeverity::P0Block
                && x.pattern == "directory depth exceeds 7")
    );
}

#[test]
fn new_file_over_100_loc_blocked() {
    let content = "fn x() {}\n".repeat(120);
    let v = detect("crates/foo/src/big.rs", &content, "Write");
    assert!(
        v.iter().any(
            |x| x.severity == NanoSeverity::P0Block && x.pattern == "new file exceeds 100 LOC"
        )
    );
}

#[test]
fn edit_over_100_loc_blocked() {
    // An Edit that pushes an existing file past 100 lines HARD-BLOCKS too:
    // it must split into the same deep hub+leaf hierarchy as a new file.
    let content = "fn x() {}\n".repeat(120);
    let v = detect("crates/foo/src/big.rs", &content, "Edit");
    assert!(
        v.iter()
            .any(|x| x.severity == NanoSeverity::P0Block && x.pattern == "file exceeds 100 LOC")
    );
}

#[test]
fn under_100_loc_passes() {
    let v = detect("crates/foo/src/small.rs", "fn x() {}\n", "Write");
    assert!(v.is_empty());
}

#[test]
fn non_rust_file_skipped() {
    let v = detect("crates/foo/src/mod.rs.txt", "anything", "Write");
    assert!(v.is_empty());
}

#[test]
fn loc_exempt_marker_in_header_allows_oversize_file() {
    // A monolithic data file (e.g. an SQL-DDL const) that declares the opt-out
    // marker up top is exempt from the LOC ceiling — it cannot be hub+leaf split.
    let mut content = String::from("//! kavach:nano-file-exempt — single SQL DDL const\n");
    content.push_str(&"const X: &str = \"...\";\n".repeat(120));
    let v = detect("crates/foo/src/schema.rs", &content, "Edit");
    assert!(
        !v.iter().any(|x| x.pattern == "file exceeds 100 LOC"),
        "header marker must exempt the LOC ceiling"
    );
}

#[test]
fn loc_exempt_marker_buried_deep_still_blocks() {
    // The marker MUST be in the header region — burying it past line 15 does not
    // exempt, so ordinary logic files cannot smuggle the marker in to escape.
    let mut content = "fn x() {}\n".repeat(40);
    content.push_str("// kavach:nano-file-exempt sneaky\n");
    content.push_str(&"fn y() {}\n".repeat(80));
    let v = detect("crates/foo/src/big.rs", &content, "Write");
    assert!(
        v.iter().any(
            |x| x.severity == NanoSeverity::P0Block && x.pattern == "new file exceeds 100 LOC"
        ),
        "a buried marker must NOT exempt"
    );
}
