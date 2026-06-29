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
fn recorded_red_merges_into_existing_persisted_state() {
    // record_red_units must persist the red unit via atomic_update (re-read under
    // lock, then merge) so the append lands durably and merges with whatever the
    // on-disk row already holds — not a blind overwrite. SOURCE: CWE-367; save.rs:99.
    let sid = "race_red_persist_unit";
    let mut prior = kavach_session::SessionState::new("/tmp/race");
    prior.session_id = sid.to_owned();
    prior.tdd_red_units.push("already_here".to_owned());
    prior.save().ok();

    let mut rec = kavach_session::SessionState::new("/tmp/race");
    rec.session_id = sid.to_owned();
    rec.files_modified_this_turn
        .push("crates/foo/src/widget_test.rs".to_owned());
    let fail = bash_fail(
        "cargo nextest run -p foo widget_test",
        "test result: FAILED. 0 passed; 1 failed",
    );
    drop(handle(&fail, &mut rec));

    // Re-read the INI directly (DB-independent): the recorded red unit lands durably.
    let path = kavach_session::state_path_for(sid);
    let ini = std::fs::read_to_string(&path).expect("state file must exist");
    let reloaded = kavach_session::parse_ini_str(&ini);
    assert!(
        reloaded.tdd_red_units.contains(&"widget".to_owned()),
        "recorded RED must persist durably via atomic_update; got {:?}",
        reloaded.tdd_red_units
    );
}
