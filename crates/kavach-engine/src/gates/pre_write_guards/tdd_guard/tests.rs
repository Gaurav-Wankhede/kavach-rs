//! Red-Green proofs for the TDD pre-write gate. Outcomes drive the code.

use super::{check, has_inline_test, production_stem_of, test_matches_unit, unit_stem};
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
    assert!(
        check(&c, &s).is_none(),
        "comment-only edit is exempt from TDD"
    );
}

#[test]
fn blocks_when_test_touched_but_not_observed_red() {
    // A touched test file WITHOUT an observed Red run is a vacuous after-the-fact
    // test — the gate must still BLOCK and direct the agent to RUN it red first.
    let s = session_with_turn_files(&["crates/foo/src/widget_test.rs"]);
    let c = ctx("crates/foo/src/widget.rs", "pub fn build() {}");
    let out = check(&c, &s).expect("touch-without-red must block");
    assert!(
        out.contains("observed-Red"),
        "names the missing red proof: {out}"
    );
    assert!(
        out.contains("NOT observed RED"),
        "directs RUN-it-red: {out}"
    );
}

#[test]
fn allows_when_test_observed_red_this_turn() {
    // The unit's test was RUN and FAILED this turn (recorded in tdd_red_units) —
    // genuine test-first. The gate must pass.
    let mut s = session_with_turn_files(&["crates/foo/src/widget_test.rs"]);
    s.tdd_red_units = vec!["widget".to_owned()];
    let c = ctx("crates/foo/src/widget.rs", "pub fn build() {}");
    assert!(check(&c, &s).is_none(), "observed-Red satisfies test-first");
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
    assert!(
        out.contains("inline test"),
        "names the inline-test violation: {out}"
    );
}

#[test]
fn has_inline_test_detects_cfg_test_module() {
    assert!(has_inline_test("fn a() {}\n#[cfg(test)]\nmod tests { }"));
    assert!(has_inline_test("#[test]\nfn t() {}"));
    assert!(!has_inline_test("pub fn build() {}\n// no tests here"));
    // A `#[path]` to an external test file is NOT an inline test.
    assert!(!has_inline_test(
        "#[path = \"widget/tests.rs\"]\nmod tests;"
    ));
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
    assert!(test_matches_unit(
        "crates/foo/src/widget_tests.rs",
        "widget"
    ));
    assert!(test_matches_unit("crates/foo/tests/widget.rs", "widget"));
    // The dominant in-engine convention: a `widget/tests.rs` subdir module paired
    // with `widget.rs` via #[path]. Must satisfy the gate.
    assert!(test_matches_unit(
        "crates/foo/src/widget/tests.rs",
        "widget"
    ));
    assert!(!test_matches_unit("crates/foo/src/other_test.rs", "widget"));
    assert!(!test_matches_unit(
        "crates/foo/src/other/tests.rs",
        "widget"
    ));
}

#[test]
fn production_stem_of_strips_test_suffix() {
    assert_eq!(
        production_stem_of("a/b/dispatch_msg_test.rs"),
        Some("dispatch_msg".to_owned())
    );
}

#[test]
fn production_stem_of_strips_tests_suffix() {
    assert_eq!(
        production_stem_of("a/b/widget_tests.rs"),
        Some("widget".to_owned())
    );
}

#[test]
fn production_stem_of_extracts_from_subdir_tests() {
    assert_eq!(
        production_stem_of("a/b/foo/tests.rs"),
        Some("foo".to_owned())
    );
}

#[test]
fn production_stem_of_extracts_from_integration_form() {
    assert_eq!(
        production_stem_of("a/tests/bar.rs"),
        Some("bar".to_owned())
    );
}

#[test]
fn production_stem_of_returns_none_for_production_file() {
    assert_eq!(production_stem_of("a/b/foo.rs"), None);
}

#[test]
fn production_stem_of_returns_none_for_bare_tests() {
    assert_eq!(production_stem_of("a/tests.rs"), None);
}
