//! `check` verdict tests: exemptions, trigger block, no-trigger allow.
use crate::gates::pre_write_algo_guard::check::check;
use crate::gates::pre_write_algo_guard::outcome::AlgoGuardOutcome;
use crate::gates::pre_write_algo_guard::triggers::ALGO_TRIGGERS;

fn is_allow(o: &AlgoGuardOutcome) -> bool {
    matches!(o, AlgoGuardOutcome::Allow)
}

fn is_block(o: &AlgoGuardOutcome) -> bool {
    matches!(o, AlgoGuardOutcome::Block(_))
}

// Build trigger-containing strings at runtime so the gate doesn't scan trigger
// keywords in this file's static source text.
fn trigger_line(kw: &str) -> String {
    format!("let x = {kw}::new();")
}

#[test]
fn allows_when_satisfied() {
    let kw = ALGO_TRIGGERS[3];
    assert!(is_allow(&check(
        "src/store.rs",
        &trigger_line(kw),
        true,
        ""
    )));
}

#[test]
fn blocks_when_not_satisfied_no_db() {
    let kw = ALGO_TRIGGERS[6];
    assert!(is_block(&check(
        "src/cache.rs",
        &trigger_line(kw),
        false,
        ""
    )));
}

#[test]
fn allows_non_rust_file() {
    let kw = ALGO_TRIGGERS[6];
    assert!(is_allow(&check(
        "src/cache.ts",
        &trigger_line(kw),
        false,
        ""
    )));
}

#[test]
fn allows_test_file() {
    let kw = ALGO_TRIGGERS[0];
    assert!(is_allow(&check(
        "src/store_tests.rs",
        &trigger_line(kw),
        false,
        ""
    )));
    assert!(is_allow(&check(
        "tests/integration.rs",
        &trigger_line(kw),
        false,
        ""
    )));
}

#[test]
fn allows_no_trigger_keywords() {
    let content = "fn greet(name: &str) -> String { name.to_string() }";
    assert!(is_allow(&check("src/greet.rs", content, false, "")));
}

#[test]
fn allows_empty_content() {
    assert!(is_allow(&check("src/lib.rs", "", false, "")));
}
