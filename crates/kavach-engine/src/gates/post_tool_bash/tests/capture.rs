//! Trajectory-capture proof: `handle` appends a Bash event to the session tape.
//!
//! This is the committed end-to-end proof that the P1 capture wiring fires —
//! `handle` → `capture_bash` → `eval_replay::capture` → the on-disk JSONL — so the
//! reward scorer has real Bash/Write witnesses (not the Stop-only tape of P0).
use crate::gates::post_tool_bash::handle;
use kavach_types::HookInput;
use std::collections::HashMap;

/// `default_trajectory_path` reads `KAVACH_HOME` from the PROCESS env, and
/// `temp_env::with_var` mutates that process-global. Cargo runs the tests in this
/// file on parallel threads, so without this lock each test's
/// `with_var` clobbers its siblings' value mid-flight: a test then reads another
/// test's home, writes its tape THERE, and counts files it does not own. That
/// produced a nondeterministic `no tape` assertion failure that passed in
/// isolation and failed in the full-workspace run. Serializing is the fix;
/// per-test UNIQUE dirs stop the tests from sharing state even when serialized.
static KAVACH_HOME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A fresh, guaranteed-empty KAVACH_HOME for one test. Caller must hold
/// [`KAVACH_HOME_LOCK`].
fn fresh_home(label: &str) -> std::path::PathBuf {
    let home = std::env::temp_dir().join(format!(
        "kavach_capture_{label}_{}_{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::remove_dir_all(&home).ok();
    std::fs::create_dir_all(&home).expect("create scratch home");
    home
}

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
    // `default_trajectory_path` roots the tape at KAVACH_HOME/.kavach/trajectories/.
    // KAVACH_HOME is the portable seam: `dirs::home_dir()` ignores env vars on
    // Windows (it calls SHGetKnownFolderPath), so setting HOME would not redirect it.
    let _guard = KAVACH_HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let home = fresh_home("e2e");

    let sid = "sess_capture_proof";
    temp_env::with_var("KAVACH_HOME", Some(home.as_os_str()), || {
        let mut session = kavach_session::SessionState::default();
        session.session_id = sid.to_owned();

        let input = bash_input("cargo check --workspace");
        // The handler swallows its own exit; we only care about the side effect.
        drop(handle(&input, &mut session));

        let tape = home
            .join(".kavach")
            .join("trajectories")
            .join(format!("{sid}.jsonl"));
        assert!(
            tape.exists(),
            "capture must create the session tape at {tape:?}"
        );

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

fn bash_input_with_output(command: &str, output: &str) -> HookInput {
    let mut input = bash_input(command);
    let mut resp = HashMap::new();
    resp.insert(
        "output".to_owned(),
        serde_json::Value::String(output.to_owned()),
    );
    input.tool_response = Some(resp);
    input
}

#[test]
fn failed_build_then_done_claim_nets_negative_through_the_oracle() {
    // THE GROUND-TRUTH ORACLE END-TO-END PROOF: the agent runs `cargo build`, it
    // FAILS (compiler error in the real output), the handler records an objective
    // Failure on the tape — then a done-claiming Stop is appended. The oracle reads
    // the recorded outcome (not the prose) and drives the score negative, killing
    // the RLAIF self-preference bias. SOURCE: decision.harness-reward-ground-truth-oracle.
    let _guard = KAVACH_HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let home = fresh_home("oracle");
    let sid = "sess_oracle_proof";
    temp_env::with_var("KAVACH_HOME", Some(home.as_os_str()), || {
        let mut session = kavach_session::SessionState::default();
        session.session_id = sid.to_owned();

        // A real build that FAILED — the runner printed a compiler error.
        let failed = bash_input_with_output(
            "cargo build --workspace",
            "error[E0308]: mismatched types\nerror: could not compile `x`",
        );
        drop(handle(&failed, &mut session));

        let tape = home
            .join(".kavach")
            .join("trajectories")
            .join(format!("{sid}.jsonl"));
        // Append the agent's false completion claim directly to the same tape.
        kavach_patterns::eval_replay::emit_to_jsonl(
            &tape,
            &kavach_patterns::eval_replay::TrajectoryEvent {
                timestamp_ms: 1,
                session_id: sid.to_owned(),
                event_kind: kavach_patterns::eval_replay::EventKind::Stop {
                    final_message: "All done — the fix is complete and the build is green."
                        .to_owned(),
                },
                outcome: None,
            },
        )
        .unwrap();

        let events = kavach_patterns::eval_replay::read_jsonl(&tape).unwrap();
        let score = kavach_patterns::reward::score_trajectory(&events);
        assert!(
            score < 0,
            "claimed-done over an objectively-failed build must net negative: {score}"
        );
    });
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn empty_session_id_writes_no_tape() {
    // The no-op guard: a missing session id must not pollute a default path.
    //
    // Assert on the tape THIS call would have produced — named `.jsonl`, since
    // `default_trajectory_path` formats `{session_id}.jsonl` and the id is
    // empty. Counting the whole directory was unsound: `KAVACH_HOME` is
    // process-global, and other tests in this binary (`post_write` capture
    // tests) legitimately write tapes through it while this test runs, so the
    // directory count raced and intermittently read 2 instead of 0. Those files
    // belong to other tests; this test's claim is exactly "an empty session id
    // writes no tape", and that is nameable.
    let _guard = KAVACH_HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let home = fresh_home("noid");
    temp_env::with_var("KAVACH_HOME", Some(home.as_os_str()), || {
        let mut session = kavach_session::SessionState::default();
        session.session_id.clear();
        drop(handle(&bash_input("cargo check"), &mut session));
        let leaked = home
            .join(".kavach")
            .join("trajectories")
            .join(".jsonl");
        assert!(
            !leaked.exists(),
            "empty session id must write no tape; found {leaked:?}"
        );
    });
    std::fs::remove_dir_all(&home).ok();
}
