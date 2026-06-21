use super::super::{MicroSeverity, detect};

#[test]
fn inline_test_module_in_production_file_blocked() {
    let content = "fn x() {}\n#[cfg(test)]\nmod tests { use super::*; }\n";
    let v = detect("crates/foo/src/ladder.rs", content, "Write");
    assert!(
        v.iter()
            .any(|x| x.severity == MicroSeverity::P0Block && x.pattern == "inline test module"),
        "a #[cfg(test)] block in a production file must P0-block"
    );
}

#[test]
fn cfg_test_mentioned_only_in_a_comment_does_not_flag() {
    // The guard's own fix message documents `#[cfg(test)]`; a naive substring
    // match would flag any file that merely mentions it in prose. Only a real
    // attribute line counts.
    let content = "//! see `#[cfg(test)]` for the convention\nfn x() {}\n";
    let v = detect("crates/foo/src/doc.rs", content, "Write");
    assert!(
        !v.iter().any(|x| x.pattern == "inline test module"),
        "a comment mentioning #[cfg(test)] must NOT flag"
    );
}

#[test]
fn path_sidecar_declaration_is_not_flagged() {
    // The PRESCRIBED fix — `#[cfg(test)] #[path = "foo_test.rs"] mod tests;` — is a
    // declaration, not an inline block, so it must PASS. Regression: the old
    // substring check flagged it, wedging every file split.
    for content in [
        "fn x() {}\n#[cfg(test)]\n#[path = \"ladder_test.rs\"]\nmod tests;\n",
        "fn x() {}\n#[cfg(test)]\nmod tests;\n",
    ] {
        let v = detect("crates/foo/src/ladder.rs", content, "Write");
        assert!(
            !v.iter().any(|x| x.pattern == "inline test module"),
            "a #[path] sidecar declaration must NOT trip the inline-test rule"
        );
    }
}

#[test]
fn inline_block_with_brace_far_below_cfg_test_is_still_caught() {
    // Reviewer edge case: the `{` opener sits many lines below `#[cfg(test)]`
    // (attributes + blank lines + a long mod path between). The forward scan
    // walks to the gated `mod` item regardless of distance — no false negative.
    let content = "fn x() {}\n\
        #[cfg(test)]\n\
        #[allow(clippy::all)]\n\
        \n\
        #[rustfmt::skip]\n\
        // a comment about the module\n\
        pub(crate) mod tests {\n\
            use super::*;\n\
        }\n";
    let v = detect("crates/foo/src/deep.rs", content, "Write");
    assert!(
        v.iter().any(|x| x.pattern == "inline test module"),
        "an inline block must be caught even with attrs/blanks before the brace"
    );
}

#[test]
fn test_sidecar_files_may_hold_the_module() {
    // The extracted homes are exempt: <name>_test.rs, tests.rs, and /tests/.
    let content = "use super::*;\n#[cfg(test)]\nmod inner {}\n";
    for path in [
        "crates/foo/src/ladder_test.rs",
        "crates/foo/src/foo/tests.rs",
        "crates/foo/tests/integration.rs",
    ] {
        let v = detect(path, content, "Write");
        assert!(
            !v.iter().any(|x| x.pattern == "inline test module"),
            "{path} is a test sidecar and must NOT trip the inline-test rule"
        );
    }
}
