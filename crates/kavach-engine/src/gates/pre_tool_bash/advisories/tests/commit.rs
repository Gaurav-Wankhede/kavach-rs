//! `check_commit_message` tests: conventional-prefix pass, missing-prefix advise,
//! non-commit ignore, and the quote-buried CWE-184 exemption.
use super::super::check_commit_message;

#[test]
fn test_commit_message_conventional_passes() {
    assert!(check_commit_message("git commit -m \"feat(auth): add token validation\"").is_none());
    assert!(check_commit_message("git commit -m \"fix: correct session reset\"").is_none());
    assert!(check_commit_message("git commit -m \"chore: bump deps\"").is_none());
    assert!(check_commit_message("git commit -m 'test: add memory guard tests'").is_none());
}

#[test]
fn test_commit_message_missing_prefix_advises() {
    assert!(check_commit_message("git commit -m \"update stuff\"").is_some());
    assert!(check_commit_message("git commit -m \"WIP changes\"").is_some());
    assert!(check_commit_message("git commit -m 'initial commit'").is_some());
}

#[test]
fn test_commit_message_non_commit_ignored() {
    assert!(check_commit_message("cargo build").is_none());
    assert!(check_commit_message("git status").is_none());
}

#[test]
fn commit_advisory_ignores_phrase_in_quoted_arg() {
    // `git commit` appearing inside another command's quoted arg is not a
    // commit invocation — must not emit [COMMIT_FORMAT].
    assert!(check_commit_message(r"rg -n 'git commit' docs/").is_none());
    assert!(check_commit_message(r#"echo "run git commit -m msg later""#).is_none());
    // A real non-conventional commit still gets the advisory.
    assert!(check_commit_message(r#"git commit -m "added stuff""#).is_some());
    // A real conventional commit still passes silently.
    assert!(check_commit_message(r#"git commit -m "feat(x): add y""#).is_none());
}
