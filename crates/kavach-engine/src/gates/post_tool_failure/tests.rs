//! Classification + self-evolve block + recurrence gate + `run` smoke coverage.
use kavach_types::HookInput;

use super::classify::{classify_failure, is_repeat_failure};
use super::rpc::{self_evolve_block, tier2_context};
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

// The noise gate: a FIRST occurrence (no prior failure marker) must stay
// silent — no [SELF_EVOLVE] research detour for a one-off mistyped flag.
#[test]
fn first_occurrence_is_not_a_repeat() {
    assert!(!is_repeat_failure("", "", "Bash", "validation"));
}

#[test]
fn same_tool_and_class_is_a_repeat() {
    assert!(is_repeat_failure(
        "Bash",
        "validation",
        "Bash",
        "validation"
    ));
}

#[test]
fn different_tool_or_class_is_not_a_repeat() {
    assert!(!is_repeat_failure(
        "Read",
        "validation",
        "Bash",
        "validation"
    ));
    assert!(!is_repeat_failure(
        "Bash",
        "transient",
        "Bash",
        "validation"
    ));
}

#[test]
fn tier2_context_carries_header_and_self_evolve() {
    let ctx = tier2_context("Bash", "3", "validation", "false", "DIAGNOSE", "boom");
    assert!(ctx.contains("[TOOL_FAILURE]"), "header present: {ctx}");
    assert!(ctx.contains("tier: research"), "tier tagged: {ctx}");
    assert!(
        ctx.contains("[SELF_EVOLVE]"),
        "research directive present: {ctx}"
    );
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
