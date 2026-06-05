//! Tests for `eval_replay`: replay, summarize, JSONL roundtrip, capture.
use super::*;

fn ev_bash(cmd: &str) -> TrajectoryEvent {
    TrajectoryEvent {
        timestamp_ms: 0,
        session_id: "t".into(),
        event_kind: EventKind::Bash {
            command: cmd.into(),
        },
    }
}
fn ev_write(path: &str, content: &str) -> TrajectoryEvent {
    TrajectoryEvent {
        timestamp_ms: 0,
        session_id: "t".into(),
        event_kind: EventKind::Write {
            file_path: path.into(),
            content: content.into(),
        },
    }
}

#[test]
fn replay_rm_rf_blocks() {
    let outs = replay_event(&ev_bash("rm -rf /"));
    assert!(
        outs.iter()
            .any(|o| o.gate == "destructive_cli_guard" && o.severity == ReplaySeverity::Block)
    );
}

#[test]
fn replay_quote_obfuscation_caught() {
    let outs = replay_event(&ev_bash("'r''m' -rf /"));
    assert!(outs.iter().any(|o| o.severity == ReplaySeverity::Block));
}

#[test]
fn replay_safe_bash_passes() {
    let outs = replay_event(&ev_bash("ls -la"));
    assert!(outs.is_empty());
}

#[test]
fn replay_migration_blocks() {
    let outs = replay_event(&ev_write(
        "migrations/0001_role.sql",
        "ALTER TABLE users ADD COLUMN role text NOT NULL;",
    ));
    assert!(
        outs.iter()
            .any(|o| o.gate == "migration_safety_guard" && o.severity == ReplaySeverity::Block)
    );
}

#[test]
fn replay_clean_handler_passes() {
    let outs = replay_event(&ev_write(
        "src/handlers/users.rs",
        "use axum;\n#[tracing::instrument(skip(repo))]\npub async fn list<R: UserRepository>(State(repo): State<Arc<R>>) {}",
    ));
    // No Block-level outcomes
    assert!(!outs.iter().any(|o| o.severity == ReplaySeverity::Block));
}

#[test]
fn replay_false_completion_detected() {
    let ev = TrajectoryEvent {
        timestamp_ms: 0,
        session_id: "t".into(),
        event_kind: EventKind::Stop {
            final_message: "All done!".into(),
        },
    };
    let outs = replay_event(&ev);
    assert!(outs.iter().any(|o| o.gate == "false_completion_detector"));
}

#[test]
fn summarize_counts_correctly() {
    let trajectory = vec![
        ev_bash("rm -rf /"), // block
        ev_bash("ls -la"),   // allow
        ev_write(
            "migrations/0001.sql",
            "ALTER TABLE u ADD COLUMN r text NOT NULL;",
        ), // block
        ev_write(
            "src/handlers/u.rs",
            "use axum; pub async fn h(State(p): State<sqlx::PgPool>) {}",
        ), // advise
    ];
    let s = summarize(&trajectory);
    assert_eq!(s.events, 4);
    assert_eq!(s.blocks, 2);
    assert_eq!(s.allows, 1);
    assert!(s.advises >= 1);
}

#[test]
fn replay_trajectory_returns_per_event_outcomes() {
    let trajectory = vec![ev_bash("rm -rf /"), ev_bash("ls -la")];
    let r = replay_trajectory(&trajectory);
    assert_eq!(r.len(), 2);
    assert!(!r[0].1.is_empty());
    assert!(r[1].1.is_empty());
}

#[test]
fn jsonl_emit_then_read_roundtrip() {
    let dir = std::env::temp_dir().join(format!("kavach_replay_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("session.jsonl");
    std::fs::remove_file(&path).ok();

    let events = vec![
        ev_bash("rm -rf /"),
        ev_bash("ls -la"),
        TrajectoryEvent {
            timestamp_ms: 42,
            session_id: "t".into(),
            event_kind: EventKind::Stop {
                final_message: "All done!".into(),
            },
        },
    ];
    for ev in &events {
        emit_to_jsonl(&path, ev).unwrap();
    }

    let read = read_jsonl(&path).unwrap();
    assert_eq!(read.len(), 3);
    assert_eq!(read[0], events[0]);
    assert_eq!(read[2].event_kind, events[2].event_kind);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn jsonl_skips_malformed_lines() {
    let dir =
        std::env::temp_dir().join(format!("kavach_replay_malformed_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("malformed.jsonl");
    std::fs::write(&path, "not-json\n{\"timestamp_ms\":1,\"session_id\":\"s\",\"event_kind\":{\"kind\":\"bash\",\"command\":\"ls\"}}\n").unwrap();
    let read = read_jsonl(&path).unwrap();
    assert_eq!(read.len(), 1);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn capture_appends_event_to_the_tape() {
    // capture() resolves the path, builds the event, and appends it.
    let dir = std::env::temp_dir().join(format!("kavach_capture_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("cap.jsonl");
    std::fs::remove_file(&path).ok();
    // Use emit_to_jsonl directly with the same kinds capture() builds — capture()
    // itself targets the home-dir default path, so we assert the kind plumbing here
    // and exercise the home-path no-op guard separately below.
    emit_to_jsonl(&path, &TrajectoryEvent {
        timestamp_ms: 7,
        session_id: "s".into(),
        event_kind: EventKind::Bash { command: "cargo check --workspace".into() },
    })
    .unwrap();
    let read = read_jsonl(&path).unwrap();
    assert_eq!(read.len(), 1);
    assert!(matches!(read[0].event_kind, EventKind::Bash { .. }));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn capture_empty_session_id_is_a_noop() {
    // No session → no tape write, no error. Guards against polluting a default file.
    assert!(capture("", 0, EventKind::Bash { command: "ls".into() }).is_ok());
}

#[test]
fn tape_survives_a_simulated_restart_and_still_replays() {
    // THE RESTART-DURABILITY PROOF. Events written to the on-disk tape in one
    // "process" are re-read by a fresh read_jsonl() (a new process after restart)
    // and replay identically — the trajectory is the durable source of truth, the
    // score is recomputed, never stored. Answers "what if the system restarts?".
    let dir = std::env::temp_dir().join(format!("kavach_restart_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("session.jsonl");
    std::fs::remove_file(&path).ok();

    // --- "process 1": capture a real working session to disk, then it dies ---
    let session = [
        EventKind::Write {
            file_path: "src/lib.rs".into(),
            content: "pub fn add(a: i64, b: i64) -> i64 { a + b }".into(),
        },
        EventKind::Bash { command: "cargo check --workspace".into() },
        EventKind::Stop { final_message: "done".into() },
    ];
    for (i, kind) in session.iter().enumerate() {
        emit_to_jsonl(&path, &TrajectoryEvent {
            timestamp_ms: i64::try_from(i).unwrap(),
            session_id: "restart".into(),
            event_kind: kind.clone(),
        })
        .unwrap();
    }

    // --- "process 2" (after restart): nothing in memory; read purely from disk ---
    let recovered = read_jsonl(&path).unwrap();
    assert_eq!(recovered.len(), 3, "all 3 events survived the restart");
    // The replay is deterministic on the recovered tape.
    let summary = summarize(&recovered);
    assert_eq!(summary.events, 3);
    // A Write + a real cargo check are present → there IS gradeable signal post-restart.
    assert!(
        matches!(recovered[0].event_kind, EventKind::Write { .. })
            && matches!(recovered[1].event_kind, EventKind::Bash { .. }),
        "Bash + Write witnesses recovered — reward has signal after restart"
    );
    std::fs::remove_dir_all(&dir).ok();
}
