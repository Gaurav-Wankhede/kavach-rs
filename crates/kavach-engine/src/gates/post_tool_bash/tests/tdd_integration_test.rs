//! Integration test: when a test file is Written, then a nextest command fails,
//! the RED is recorded. Tests the full path from Write hook → files_modified_this_turn →
//! PostToolUse:Bash record_red_units.
use crate::gates::post_tool_bash::handle;
use crate::gates::post_write;
use kavach_types::HookInput;
use std::collections::HashMap;

fn write_input(file_path: &str, content: &str) -> HookInput {
    let mut tool_input = HashMap::new();
    tool_input.insert(
        "file_path".to_owned(),
        serde_json::Value::String(file_path.to_owned()),
    );
    tool_input.insert(
        "content".to_owned(),
        serde_json::Value::String(content.to_owned()),
    );
    HookInput {
        tool_name: "Write".to_owned(),
        tool_input: Some(tool_input),
        ..HookInput::default()
    }
}

fn bash_input_with_output(command: &str, output: &str) -> HookInput {
    let mut tool_input = HashMap::new();
    tool_input.insert(
        "command".to_owned(),
        serde_json::Value::String(command.to_owned()),
    );
    let mut resp = HashMap::new();
    resp.insert(
        "output".to_owned(),
        serde_json::Value::String(output.to_owned()),
    );
    HookInput {
        tool_name: "Bash".to_owned(),
        tool_input: Some(tool_input),
        tool_response: Some(resp),
        ..HookInput::default()
    }
}

#[test]
fn write_test_file_then_nextest_fail_records_red_integration() {
    let mut session = kavach_session::SessionState::default();
    session.session_id = "test_integration".to_owned();

    // Step 1: Write a test file
    let write_input = write_input(
        "crates/kavach-cli/src/cmd/audit/lens/selection_test.rs",
        "#[test]\nfn test_selection() {\n    let x = selection::greet();\n}",
    );
    post_write::session::advance_session(&mut session, "crates/kavach-cli/src/cmd/audit/lens/selection_test.rs");

    assert!(
        session.files_modified_this_turn.contains(&"crates/kavach-cli/src/cmd/audit/lens/selection_test.rs".to_owned()),
        "Write should populate files_modified_this_turn; got {:?}",
        session.files_modified_this_turn
    );

    // Step 2: Run nextest, which fails (compile error)
    let compile_fail_output = "error[E0432]: cannot find module `selection` in this crate\n\
         --> crates/kavach-cli/src/cmd/audit/lens/selection_test.rs:1:5\n\
         error: could not compile `kavach-cli`";

    let bash_input = bash_input_with_output(
        "cargo nextest run -p kavach-cli audit::lens::selection_test",
        compile_fail_output,
    );
    drop(handle(&bash_input, &mut session));

    // Step 3: Verify RED was recorded
    assert!(
        session.tdd_red_units.contains(&"selection".to_owned()),
        "compile-error on selection_test must record production stem 'selection' as RED; got {:?}",
        session.tdd_red_units
    );
}
