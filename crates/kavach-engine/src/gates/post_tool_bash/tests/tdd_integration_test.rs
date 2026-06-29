//! Integration test: when a test file is Written, then a nextest command fails,
//! the RED is recorded. Tests the full path from Write hook → files_modified_this_turn →
//! PostToolUse:Bash record_red_units.
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
fn recorded_red_survives_a_concurrent_blind_save_of_stale_state() {
    // ROOT CAUSE of the TDD gate's "never satisfiable" bug: record_red_units used a
    // blind save() that lost its tdd_red_units write when a parallel hook saved a
    // stale (pre-recorder) snapshot. The recorder must persist via atomic_update so
    // the red-unit append re-reads under lock and merges. SOURCE: CWE-367; save.rs:99.
    let sid = "race_red_persist_unit";
    // A stale in-memory snapshot taken BEFORE the recorder (empty tdd_red_units),
    // standing in for a parallel hook that loaded the pre-recorder state.
    let mut stale = kavach_session::SessionState::new("/tmp/race");
    stale.session_id = sid.to_owned();
    stale.save().ok();

    // Recorder loads its own copy, records RED, persists (the unit under fix).
    let mut rec = kavach_session::SessionState::new("/tmp/race");
    rec.session_id = sid.to_owned();
    rec.files_modified_this_turn
        .push("crates/foo/src/widget_test.rs".to_owned());
    handle::record_red_units_for_test(&mut rec);

    // The stale hook blind-saves AFTER the recorder — the lost-update window. A
    // merge-under-lock persist must NOT let this erase the recorded red unit.
    stale.save().ok();

    let reloaded =
        kavach_session::load_session_state_for(sid).expect("session row must persist");
    assert!(
        reloaded.tdd_red_units.contains(&"widget".to_owned()),
        "recorded RED must survive a concurrent stale blind-save; got {:?}",
        reloaded.tdd_red_units
    );
}
#[test]
fn write_test_file_then_nextest_fail_records_red_integration() {
    let mut session = kavach_session::SessionState::default();
    session.session_id = "test_integration".to_owned();

    // Step 1: Simulate what PostWrite:session hook does — populate files_modified_this_turn
    let test_file_path = "crates/kavach-cli/src/cmd/audit/lens/selection_test.rs";
    session.files_modified_this_turn.push(test_file_path.to_owned());

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
