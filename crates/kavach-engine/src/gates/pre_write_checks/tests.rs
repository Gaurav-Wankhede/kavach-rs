//! Write-context extraction, code/test-file classification, and bulk-checkbox
//! detection (md-only, 10+ threshold, content vs `new_str` source).
use super::checkbox::detect_bulk_checkbox;
use super::classify::{is_code_write, is_test_or_exempt};
use super::context::extract_write_context;
use kavach_types::HookInput;

#[test]
fn test_extract_write_context() {
    let input = HookInput {
        tool_input: Some(std::collections::HashMap::from([
            ("file_path".into(), serde_json::json!("src/main.rs")),
            ("content".into(), serde_json::json!("fn main() {}")),
        ])),
        ..Default::default()
    };
    let ctx = extract_write_context(&input);
    assert!(ctx.contains("src/main.rs"));
    assert!(ctx.contains("fn main"));
}

#[test]
fn test_is_code_write() {
    assert!(is_code_write("src/main.rs"));
    assert!(!is_code_write("README.md"));
}

#[test]
fn test_bulk_checkbox_warned() {
    let many_checked = "- [x] a\n".repeat(15);
    assert!(detect_bulk_checkbox("plan.md", &many_checked, "").is_some());
}

#[test]
fn test_bulk_checkbox_few_ok() {
    let few = "- [x] a\n- [x] b\n- [x] c\n";
    assert!(detect_bulk_checkbox("plan.md", few, "").is_none());
}

#[test]
fn test_bulk_checkbox_non_md_ok() {
    let many = "- [x] a\n".repeat(20);
    assert!(detect_bulk_checkbox("src/main.rs", &many, "").is_none());
}

#[test]
fn test_bulk_checkbox_edit_new_str() {
    let many = "- [X] done\n".repeat(12);
    assert!(detect_bulk_checkbox("docs/plan.md", "", &many).is_some());
}

#[test]
fn test_is_test_or_exempt() {
    assert!(is_test_or_exempt("src/gates/intent_tests.rs"));
    assert!(is_test_or_exempt("tests/integration.rs"));
    assert!(is_test_or_exempt("Cargo.toml"));
    assert!(is_test_or_exempt("README.md"));
    assert!(is_test_or_exempt("CLAUDE.md"));
    assert!(!is_test_or_exempt("src/gates/intent.rs"));
}
