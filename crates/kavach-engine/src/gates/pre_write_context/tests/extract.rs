//! Content / `new_string` extraction, precedence, and file categorization.
use crate::gates::pre_write_context::WriteContext;
use kavach_types::HookInput;

#[test]
fn should_extract_content_from_write_tool() {
    let mut input = HookInput::default();
    input.tool_name = "Write".into();
    input.tool_input = Some(std::collections::HashMap::from([
        ("file_path".into(), serde_json::json!("src/main.rs")),
        ("content".into(), serde_json::json!("fn main() {}")),
    ]));
    let ctx = WriteContext::extract(&input);
    assert_eq!(ctx.file_path, "src/main.rs");
    assert_eq!(ctx.content, "fn main() {}");
    assert!(ctx.is_code);
    assert!(ctx.is_rust);
    assert!(!ctx.is_frontend);
    assert!(!ctx.is_test);
}

#[test]
fn should_extract_new_string_from_edit_tool() {
    let mut input = HookInput::default();
    input.tool_name = "Edit".into();
    input.tool_input = Some(std::collections::HashMap::from([
        ("file_path".into(), serde_json::json!("src/App.tsx")),
        ("new_string".into(), serde_json::json!("export default App")),
    ]));
    let ctx = WriteContext::extract(&input);
    assert_eq!(ctx.content, "export default App");
    assert!(ctx.is_frontend);
    assert!(!ctx.is_rust);
}

#[test]
fn should_prefer_content_over_new_string() {
    let mut input = HookInput::default();
    input.tool_name = "Write".into();
    input.tool_input = Some(std::collections::HashMap::from([
        ("file_path".into(), serde_json::json!("src/lib.rs")),
        ("content".into(), serde_json::json!("// content")),
        ("new_string".into(), serde_json::json!("// new_string")),
    ]));
    let ctx = WriteContext::extract(&input);
    assert_eq!(ctx.content, "// content");
}

#[test]
fn should_detect_test_file() {
    let mut input = HookInput::default();
    input.tool_input = Some(std::collections::HashMap::from([
        (
            "file_path".into(),
            serde_json::json!("src/gates/intent_tests.rs"),
        ),
        ("content".into(), serde_json::json!("test code")),
    ]));
    let ctx = WriteContext::extract(&input);
    assert!(ctx.is_test);
}
