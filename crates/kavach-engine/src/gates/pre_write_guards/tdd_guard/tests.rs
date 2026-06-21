//! Red-Green proofs for the TDD pre-write gate. Outcomes drive the code.

use super::{check, has_inline_test, unit_stem, test_matches_unit};
use crate::gates::pre_write_context::WriteContext;
use kavach_session::SessionState;

fn ctx<'a>(path: &'a str, content: &'a str) -> WriteContext<'a> {
    WriteContext {
        file_path: path,
        tool_name: "Write",
        content,
        effective_content: content.to_owned(),
        is_code: kavach_patterns::is_rust_file(path),
        is_test: kavach_patterns::is_test_file(path),
        is_rust: kavach_patterns::is_rust_file(path),
        is_frontend: false,
    }
}

fn session_with_turn_files(files: &[&str]) -> SessionState {
    let mut s = SessionState::default();
    s.files_modified_this_turn = files.iter().map(|f| (*f).to_owned()).collect();
    s
}

#[test]
fn blocks_production_code_when_no_test_came_first() {
    let s = SessionState::default();
    let c = ctx("crates/foo/src/widget.rs", "pub fn build() {}");
    let out = check(&c, &s).expect("must block: code without a prior test");
    assert!(out.contains("[TDD"), "block reason is tagged: {out}");
}

#[test]
fn allows_comment_only_change_without_a_test() {
    // A comment/doc edit changes no executable code — TDD must NOT block it, so
    // the 444-file comment sweep can proceed without faking a test per file.
    let s = SessionState::default();
    let c = ctx(
        "crates/foo/src/widget.rs",
        "// just a tightened comment\n// another line\n",
    );
    assert!(check(&c, &s).is_none(), "comment-only edit is exempt from TDD");
}

#[test]
fn allows_when_matching_test_touched_first_this_turn() {
    // The unit's sibling test file was written EARLIER this turn -> Red came first.
    let s = session_with_turn_files(&["crates/foo/src/widget_test.rs"]);
    let c = ctx("crates/foo/src/widget.rs", "pub fn build() {}");
    assert!(check(&c, &s).is_none(), "test-first satisfies the gate");
}

#[test]
fn blocks_inline_test_in_production_file() {
    // Inline #[test] in a production file is FORBIDDEN — tests live in a separate
    // mapped file. An in-file test must NOT satisfy the gate; it must block.
    let s = session_with_turn_files(&["crates/foo/src/widget/tests.rs"]);
    let c = ctx(
        "crates/foo/src/widget.rs",
        "pub fn build() {}\n#[cfg(test)]\nmod tests { #[test] fn t() {} }",
    );
    let out = check(&c, &s).expect("inline test must block");
    assert!(out.contains("inline test"), "names the inline-test violation: {out}");
}

#[test]
fn has_inline_test_detects_cfg_test_module() {
    assert!(has_inline_test("fn a() {}\n#[cfg(test)]\nmod tests { }"));
    assert!(has_inline_test("#[test]\nfn t() {}"));
    assert!(!has_inline_test("pub fn build() {}\n// no tests here"));
    // A `#[path]` to an external test file is NOT an inline test.
    assert!(!has_inline_test("#[path = \"widget/tests.rs\"]\nmod tests;"));
}

#[test]
fn allows_writing_the_test_file_itself() {
    let s = SessionState::default();
    let c = ctx("crates/foo/src/widget_test.rs", "#[test] fn t() {}");
    assert!(check(&c, &s).is_none(), "the test write is never blocked");
}

#[test]
fn allows_non_code_files() {
    let s = SessionState::default();
    let c = ctx("README.md", "# docs");
    assert!(check(&c, &s).is_none(), "non-code is exempt");
}

#[test]
fn bypass_env_disables_the_gate() {
    if std::env::var_os("KAVACH_TDD_BYPASS").is_some() {
        let s = SessionState::default();
        let c = ctx("crates/foo/src/widget.rs", "pub fn build() {}");
        assert!(check(&c, &s).is_none());
    }
}

#[test]
fn unit_stem_strips_dir_and_extension() {
    assert_eq!(unit_stem("crates/foo/src/widget.rs"), "widget");
    assert_eq!(unit_stem("bare.rs"), "bare");
}

#[test]
fn test_matches_unit_recognizes_sibling_conventions() {
    assert!(test_matches_unit("crates/foo/src/widget_test.rs", "widget"));
    assert!(test_matches_unit("crates/foo/src/widget_tests.rs", "widget"));
    assert!(test_matches_unit("crates/foo/tests/widget.rs", "widget"));
    // The dominant in-engine convention: a `widget/tests.rs` subdir module paired
    // with `widget.rs` via #[path]. Must satisfy the gate.
    assert!(test_matches_unit("crates/foo/src/widget/tests.rs", "widget"));
    assert!(!test_matches_unit("crates/foo/src/other_test.rs", "widget"));
    assert!(!test_matches_unit("crates/foo/src/other/tests.rs", "widget"));
}
