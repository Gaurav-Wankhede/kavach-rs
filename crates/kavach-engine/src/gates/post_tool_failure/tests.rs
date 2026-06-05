//! Classification + self-evolve block + `run` smoke coverage.
use kavach_types::HookInput;

use super::classify::classify_failure;
use super::rpc::self_evolve_block;
use super::run;

#[test]
fn classify_transient() {
    assert_eq!(classify_failure("rate limit exceeded"), "transient");
    assert_eq!(classify_failure("connection timed out"), "transient");
}

#[test]
fn classify_not_found() {
    assert_eq!(classify_failure("No such file or directory"), "not_found");
    assert_eq!(classify_failure("resource does not exist"), "not_found");
}

#[test]
fn classify_permission() {
    assert_eq!(classify_failure("permission denied"), "permission");
    assert_eq!(classify_failure("403 Forbidden"), "permission");
}

#[test]
fn classify_validation() {
    assert_eq!(classify_failure("invalid argument"), "validation");
    assert_eq!(classify_failure("syntax error"), "validation");
}

#[test]
fn self_evolve_block_contains_fingerprint() {
    let block = self_evolve_block("source .env loads secret values", "Bash", "validation");
    assert!(block.contains("[SELF_EVOLVE]"));
    assert!(block.contains("novel_error"));
    assert!(block.contains("env")); // tokenized fingerprint present
}

#[test]
fn run_exits_without_panic_on_default_input() {
    let input = HookInput {
        tool_name: "Bash".into(),
        error: "No such file or directory".into(),
        ..Default::default()
    };
    assert!(run(&input).is_ok());
}
