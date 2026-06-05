//! Trajectory-capture proof: `handle` appends a Bash event to the session tape.
//!
//! This is the committed end-to-end proof that the P1 capture wiring fires —
//! `handle` → `capture_bash` → `eval_replay::capture` → the on-disk JSONL — so the
//! reward scorer has real Bash/Write witnesses (not the Stop-only tape of P0).
use crate::gates::post_tool_bash::handle;
use kavach_types::HookInput;
use std::collections::HashMap;

fn bash_input(command: &str) -> HookInput {
    let mut tool_input = HashMap::new();
    tool_input.insert(
        "command".to_owned(),
        serde_json::Value::String(command.to_owned()),
    );
    HookInput {
        tool_name: "Bash".to_owned(),
        tool_input: Some(tool_input),
        ..HookInput::default()
    }
}

#[test]
fn handle_appends_a_bash_event_to_the_session_tape() {
    // Point HOME at a temp dir so the tape lands somewhere we can read + clean up.
    // `default_trajectory_path` is HOME-rooted (~/.kavach/trajectories/<sid>.jsonl).
    let home = std::env::temp_dir().join(format!("kavach_capture_e2e_{}", std::process::id()));
    std::fs::create_dir_all(&home).unwrap();

    let sid = "sess_capture_proof";
    temp_env::with_var("HOME", Some(home.as_os_str()), || {
        let mut session = kavach_session::SessionState::default();
        session.session_id = sid.to_owned();

        let input = bash_input("cargo check --workspace");
        // The handler swallows its own exit; we only care about the side effect.
        drop(handle(&input, &mut session));

        let tape = home
            .join(".kavach")
            .join("trajectories")
            .join(format!("{sid}.jsonl"));
        assert!(tape.exists(), "capture must create the session tape at {tape:?}");

        let events = kavach_patterns::eval_replay::read_jsonl(&tape).unwrap();
        assert!(
            events.iter().any(|e| matches!(
                &e.event_kind,
                kavach_patterns::eval_replay::EventKind::Bash { command }
                    if command.contains("cargo check")
            )),
            "the tape must carry the Bash(cargo check) event — got {events:?}"
        );
        // And the reward now scores it as a real build witness (non-zero).
        let score = kavach_patterns::reward::score_trajectory(&events);
        assert!(score > 0, "a real cargo check must score positive: {score}");
    });

    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn empty_session_id_writes_no_tape() {
    // The no-op guard: a missing session id must not pollute a default path.
    let home = std::env::temp_dir().join(format!("kavach_capture_noid_{}", std::process::id()));
    std::fs::create_dir_all(&home).unwrap();
    temp_env::with_var("HOME", Some(home.as_os_str()), || {
        let mut session = kavach_session::SessionState::default();
        session.session_id.clear();
        drop(handle(&bash_input("cargo check"), &mut session));
        let dir = home.join(".kavach").join("trajectories");
        let count = std::fs::read_dir(&dir).map_or(0, Iterator::count);
        assert_eq!(count, 0, "no session id → no tape");
    });
    std::fs::remove_dir_all(&home).ok();
}
