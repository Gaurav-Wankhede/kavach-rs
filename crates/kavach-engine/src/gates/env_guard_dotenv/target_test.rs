//! Sidecar tests for `target` (micro-file rule: no inline tests).
use super::*;

#[test]
fn plain_dotenv_read_is_a_target() {
    assert!(targets_dotenv_file("cat .env"));
    assert!(targets_dotenv_file("rg secret .env"));
    assert!(targets_dotenv_file("cat .env.production"));
    assert!(targets_dotenv_file("source .envrc"));
    assert!(targets_dotenv_file("cat ./.env.local"));
}

#[test]
fn repo_search_with_named_source_root_is_not_a_target() {
    // The observed false positive: `.env` is a search root alongside a NAMED dir
    // (`crates`/`src`). A named root is real evidence of a multi-root repo grep.
    assert!(!targets_dotenv_file("rg -l pat crates .env --type rust"));
    assert!(!targets_dotenv_file("rg dotenv src .env"));
    assert!(!targets_dotenv_file("rg KEY services .env"));
}

#[test]
fn dot_root_with_dotenv_now_blocks() {
    // Fail-OPEN fix: `.`/`./`/`..` are NO LONGER source roots. A lone `.` is the
    // default search root and routinely co-occurs with a genuine `.env` target, so
    // `rg secret . .env` / `grep -r SECRET . .env` is a real dotenv read — BLOCK it.
    assert!(targets_dotenv_file("rg secret . .env"));
    assert!(targets_dotenv_file("grep -r secret . .env"));
    assert!(targets_dotenv_file("grep -r secret ./ .env"));
}

#[test]
fn dotenv_substring_in_longer_token_is_not_a_target() {
    assert!(!targets_dotenv_file("cat crates/.env-fixtures/data.txt"));
    assert!(!targets_dotenv_file("rg pat app.environment.ts"));
    assert!(!targets_dotenv_file("cat my.env.example.md")); // basename `my.env.example.md` ≠ dotenv
}

#[test]
fn no_dotenv_token_at_all() {
    assert!(!targets_dotenv_file("rg PORT config.toml"));
    assert!(!targets_dotenv_file("ls -la"));
    assert!(!targets_dotenv_file(""));
}

#[test]
fn flag_carrying_dotenv_value_is_a_target() {
    assert!(targets_dotenv_file("dotenv --env-file=.env run cmd"));
}
