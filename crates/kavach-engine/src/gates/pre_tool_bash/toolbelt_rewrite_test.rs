//! Unit tests for toolbelt rewriting.
use super::*;

#[test]
fn test_grep_rewrites_to_rg() {
    assert_eq!(
        rewrite_toolbelt("grep -rn foo src/").as_deref(),
        Some("rg -rn foo src/")
    );
    assert_eq!(
        rewrite_toolbelt("grep foo").as_deref(),
        Some("rg foo")
    );
}

#[test]
fn test_cat_rewrites_to_bat() {
    assert_eq!(
        rewrite_toolbelt("cat file.txt").as_deref(),
        Some("bat --paging=never file.txt")
    );
}

#[test]
fn test_find_rewrites_to_fd() {
    assert_eq!(
        rewrite_toolbelt("find . -name '*.rs'").as_deref(),
        Some("fd . -name '*.rs'")
    );
}

#[test]
fn test_jq_rewrites_to_jaq() {
    assert_eq!(
        rewrite_toolbelt("jq '.name' pkg.json").as_deref(),
        Some("jaq '.name' pkg.json")
    );
}

#[test]
fn test_pipeline_rewrites_multiple_tools() {
    assert_eq!(
        rewrite_toolbelt("cat Cargo.toml | grep version").as_deref(),
        Some("bat --paging=never Cargo.toml | rg version")
    );
}

#[test]
fn test_compound_commands_rewrites() {
    assert_eq!(
        rewrite_toolbelt("ls -la && tree src").as_deref(),
        Some("eza -la && erd src")
    );
}

#[test]
fn test_quoted_text_not_treated_as_command() {
    assert!(rewrite_toolbelt("echo 'grep foo'").is_none());
    assert!(rewrite_toolbelt("echo \"cat file\"").is_none());
}

#[test]
fn test_env_prefix_preserved() {
    assert_eq!(
        rewrite_toolbelt("DEBUG=1 cat log.txt").as_deref(),
        Some("DEBUG=1 bat --paging=never log.txt")
    );
}

#[test]
fn test_substring_word_not_rewritten() {
    assert!(rewrite_toolbelt("category list").is_none());
    assert!(rewrite_toolbelt("findstr pattern").is_none());
}
