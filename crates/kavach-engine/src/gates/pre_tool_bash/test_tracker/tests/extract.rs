//! `extract_cargo_job_key` parsing + CWE-184 quote-awareness tests.
use crate::gates::pre_tool_bash::test_tracker::extract::extract_cargo_job_key;

#[test]
fn should_extract_crate_key_from_p_flag() {
    assert_eq!(
        extract_cargo_job_key("cargo test -p kavach-engine --lib"),
        Some("kavach-engine".into())
    );
}

#[test]
fn should_extract_workspace_key_when_no_p_flag() {
    assert_eq!(
        extract_cargo_job_key("cargo test --workspace"),
        Some("__workspace__".into())
    );
    assert_eq!(
        extract_cargo_job_key("cargo test 2>&1 | tail -20"),
        Some("__workspace__".into())
    );
}

#[test]
fn build_and_check_are_now_tracked_jobs() {
    // The fix: `build`/`check` share the same `target/` lock as `test`, so a
    // duplicate concurrent invocation must be detectable. These previously
    // returned None (the gap that let two `cargo build` shells race).
    assert_eq!(
        extract_cargo_job_key("cargo build --release -p kavach-cli"),
        Some("kavach-cli".into())
    );
    assert_eq!(
        extract_cargo_job_key("cargo build --release"),
        Some("__workspace__".into())
    );
    assert_eq!(
        extract_cargo_job_key("cargo check --workspace --all-targets"),
        Some("__workspace__".into())
    );
    assert_eq!(
        extract_cargo_job_key("cargo check -p kavach-engine"),
        Some("kavach-engine".into())
    );
}

#[test]
fn should_return_none_for_non_cargo_job_command() {
    assert_eq!(extract_cargo_job_key("git status"), None);
    assert_eq!(extract_cargo_job_key("cargo fmt --all -- --check"), None);
    assert_eq!(extract_cargo_job_key("cargo clippy -- -D warnings"), None);
}

#[test]
fn should_not_match_test_keyword_in_content_body() {
    // Regression: kavach db write --content "...cargo test..." must NOT trigger
    assert_eq!(
        extract_cargo_job_key(
            r#"kavach db write --project foo --content "6 files pending cargo test execution""#
        ),
        None
    );
    assert_eq!(
        extract_cargo_job_key(
            r#"kavach db write --content "run cargo test -p service-soundbak next session""#
        ),
        None
    );
}

#[test]
fn should_not_match_phrase_in_quoted_arg_of_other_tool() {
    // Regression (CWE-184): the literal phrase inside ANOTHER tool's quoted
    // argument is DATA, not a cargo invocation.
    assert_eq!(
        extract_cargo_job_key(r"rg -n 'UNSCOPED|cargo nextest' crates/"),
        None
    );
    assert_eq!(
        extract_cargo_job_key(r#"grep "cargo test" build.log"#),
        None
    );
    assert_eq!(
        extract_cargo_job_key(r#"echo "remember to cargo nextest run later""#),
        None
    );
    assert_eq!(
        extract_cargo_job_key(r#"git commit -m "fix: cargo test was flaky in CI""#),
        None
    );
    // A quoted `cargo build` mention must NOT register either (new coverage).
    assert_eq!(
        extract_cargo_job_key(r#"echo "next: cargo build --release""#),
        None
    );
}

#[test]
fn should_still_detect_real_cargo_test_forms() {
    // Command-position detection: bare, piped, VAR= prefix, &&-chained.
    assert_eq!(
        extract_cargo_job_key("cargo test 2>&1 | tail"),
        Some("__workspace__".into())
    );
    assert_eq!(
        extract_cargo_job_key("RUST_LOG=debug cargo nextest run"),
        Some("__workspace__".into())
    );
    assert_eq!(
        extract_cargo_job_key("cargo build && cargo test -p kavach-db"),
        Some("kavach-db".into())
    );
}

#[test]
fn quoted_pipe_does_not_split_into_fake_cargo_segment() {
    // §RADIUS-INTEGRITY CWE-184: a literal | inside a quoted arg must not
    // split the command into a fake `cargo nextest` segment.
    assert_eq!(
        extract_cargo_job_key("rg -n 'foo|cargo nextest' src/"),
        None
    );
    assert_eq!(
        extract_cargo_job_key(r"rg -n 'PASS|cargo test' build.log"),
        None
    );
    assert_eq!(
        extract_cargo_job_key(r"grep 'a|cargo test|b' migrations/"),
        None
    );
    // Real cargo invocations chained AFTER a quoted arg still classify.
    assert_eq!(
        extract_cargo_job_key("rg 'foo' src/ && cargo test -p kavach-engine"),
        Some("kavach-engine".into())
    );
}
