//! `strip_string_literals` tests: drop quoted triggers, keep code triggers.
use crate::gates::pre_write_algo_guard::strip::strip_string_literals;
use crate::gates::pre_write_algo_guard::triggers::ALGO_TRIGGERS;

#[test]
fn strip_literals_removes_trigger_in_string() {
    let kw = ALGO_TRIGGERS[3];
    let content = format!(r#"const NAMES: &[&str] = &["{kw}"];"#);
    let stripped = strip_string_literals(&content);
    assert!(!stripped.contains(kw));
}

#[test]
fn strip_literals_preserves_code_trigger() {
    let kw = ALGO_TRIGGERS[3];
    let content = format!("let m = {kw}::new();");
    let stripped = strip_string_literals(&content);
    assert!(stripped.contains(kw));
}
