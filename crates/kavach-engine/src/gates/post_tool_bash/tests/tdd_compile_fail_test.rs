//! TDD red-recording proof: compile-error on a brand-new test file records RED.
//! When authoring a new unit, its test file can only compile-fail initially
//! (the production module doesn't exist), never test-fail. This test proves
//! that a `cargo nextest run` exit non-zero on a COMPILE error is classified
//! as Failure and recorded as RED in the session.
use crate::gates::post_tool_bash::handle;
use kavach_types::HookInput;
use std::collections::HashMap;

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
fn cargo_nextest_compile_error_classifies_as_failure() {
    let mut session = kavach_session::SessionState::default();
    session.session_id = "test_compile_err".to_owned();
    session.files_modified_this_turn.push("src/foo_test.rs".to_owned());

    let compile_fail_output = "error[E0433]: cannot find module `foo` in this crate\n\
         --> src/foo_test.rs:1:5\n\
          |\n\
         1 | mod foo;\n\
           |     ^^^ not found in this crate\n\
         error: could not compile `my-crate` (bin test)";

    let input = bash_input_with_output(
        "cargo nextest run -p my-crate foo_test",
        compile_fail_output,
    );
    drop(handle(&input, &mut session));

    assert!(
        session.tdd_red_units.contains(&"foo".to_owned()),
        "compile-error on foo_test must record its production stem (foo) RED in tdd_red_units; got {:?}",
        session.tdd_red_units
    );
}

#[test]
fn cargo_test_compile_error_classifies_as_failure() {
    let mut session = kavach_session::SessionState::default();
    session.session_id = "test_compile_err2".to_owned();
    session.files_modified_this_turn.push("src/bar_test.rs".to_owned());

    let compile_fail_output = "error[E0432]: unresolved import `bar`\n\
         --> src/bar_test.rs:5:5\n\
          |\n\
         5 | use bar::baz;\n\
           |     ^^^ could not find `bar` in this crate\n\
         error: could not compile `my-crate`";

    let input = bash_input_with_output(
        "cargo test -p my-crate bar_test",
        compile_fail_output,
    );
    drop(handle(&input, &mut session));

    assert!(
        session.tdd_red_units.contains(&"bar".to_owned()),
        "compile-error on bar_test must record its production stem (bar) RED; got {:?}",
        session.tdd_red_units
    );
}

#[test]
fn cargo_nextest_with_no_output_on_compile_fail_records_red() {
    let mut session = kavach_session::SessionState::default();
    session.session_id = "test_compile_err3".to_owned();
    session.files_modified_this_turn.push("src/baz_test.rs".to_owned());

    let input = bash_input_with_output(
        "cargo nextest run -p my-crate baz_test",
        "",
    );
    drop(handle(&input, &mut session));

    assert!(
        session.tdd_red_units.is_empty(),
        "empty output with no host_error flag should NOT record RED; got {:?}",
        session.tdd_red_units
    );
}

#[test]
fn cargo_nextest_with_host_error_flag_on_compile_fail_records_red() {
    let mut session = kavach_session::SessionState::default();
    session.session_id = "test_compile_err4".to_owned();
    session.files_modified_this_turn.push("src/qux_test.rs".to_owned());

    let mut tool_input = HashMap::new();
    tool_input.insert(
        "command".to_owned(),
        serde_json::Value::String("cargo nextest run -p my-crate qux_test".to_owned()),
    );
    let mut resp = HashMap::new();
    resp.insert(
        "output".to_owned(),
        serde_json::Value::String(String::new()),
    );
    resp.insert(
        "is_error".to_owned(),
        serde_json::Value::Bool(true),
    );
    let input = HookInput {
        tool_name: "Bash".to_owned(),
        tool_input: Some(tool_input),
        tool_response: Some(resp),
        ..HookInput::default()
    };
    drop(handle(&input, &mut session));

    assert!(
        session.tdd_red_units.contains(&"qux".to_owned()),
        "host_error flag (no output needed) must record RED; got {:?}",
        session.tdd_red_units
    );
}
