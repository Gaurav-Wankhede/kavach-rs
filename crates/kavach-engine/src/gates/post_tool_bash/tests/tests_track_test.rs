//! Persistence proofs for the Red-unit recorder (`tests_track::record_red_units`).
use crate::gates::post_tool_bash::handle::handle;
use kavach_types::HookInput;
use std::collections::HashMap;

fn bash_fail(command: &str, output: &str) -> HookInput {
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
    let mut stale = kavach_session::SessionState::new("/tmp/race");
    stale.session_id = sid.to_owned();
    stale.save().ok();

    let mut rec = kavach_session::SessionState::new("/tmp/race");
    rec.session_id = sid.to_owned();
    rec.files_modified_this_turn
        .push("crates/foo/src/widget_test.rs".to_owned());
    let fail = bash_fail(
        "cargo nextest run -p foo widget_test",
        "test result: FAILED. 0 passed; 1 failed",
    );
    drop(handle(&fail, &mut rec));

    // The stale hook blind-saves AFTER the recorder — the lost-update window.
    stale.save().ok();

    let reloaded =
        kavach_session::load_session_state_for(sid).expect("session row must persist");
    assert!(
        reloaded.tdd_red_units.contains(&"widget".to_owned()),
        "recorded RED must survive a concurrent stale blind-save; got {:?}",
        reloaded.tdd_red_units
    );
}
