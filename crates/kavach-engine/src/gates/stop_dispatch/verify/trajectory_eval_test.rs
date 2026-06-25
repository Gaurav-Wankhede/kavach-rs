use kavach_patterns::eval_replay::{EventKind, TrajectoryEvent};

#[test]
fn path_is_gate_or_dispatch_true_gates() {
    assert!(super::path_is_gate_or_dispatch("crates/kavach-engine/src/gates/foo.rs"));
}

#[test]
fn path_is_gate_or_dispatch_true_stop_dispatch() {
    assert!(super::path_is_gate_or_dispatch(
        "crates/kavach-engine/src/gates/stop_dispatch/verify.rs"
    ));
}

#[test]
fn path_is_gate_or_dispatch_true_reward_rubric() {
    assert!(super::path_is_gate_or_dispatch(
        "crates/kavach-patterns/src/reward.rs"
    ));
}

#[test]
fn path_is_gate_or_dispatch_false_unrelated() {
    assert!(!super::path_is_gate_or_dispatch(
        "crates/kavach-engine/src/foo/bar.rs"
    ));
}

#[test]
fn path_is_gate_or_dispatch_false_reward_preset() {
    assert!(!super::path_is_gate_or_dispatch(
        "crates/kavach-patterns/src/reward/presets.rs"
    ));
}

#[test]
fn eval_advisory_positive_score_returns_none() {
    let events = vec![TrajectoryEvent {
        timestamp_ms: 1000,
        session_id: "test".to_owned(),
        event_kind: EventKind::Write {
            file_path: "src/lib.rs".to_owned(),
            content: "#[test]\nfn test_foo() {}".to_owned(),
        },
        outcome: None,
    }];
    let result = super::eval_trajectory_score(&events);
    assert!(result >= 0, "single write + substantive test should be positive");
}

#[test]
fn eval_advisory_negative_score_returns_some() {
    let events = vec![TrajectoryEvent {
        timestamp_ms: 1000,
        session_id: "test".to_owned(),
        event_kind: EventKind::Stop {
            final_message: "starting the next step is yours to run".to_owned(),
        },
        outcome: None,
    }];
    let result = super::eval_trajectory_score(&events);
    assert!(
        result < 0,
        "deferral handoff should be negative, got {}",
        result
    );
}

#[test]
fn eval_advisory_multiple_negatives_strong_penalty() {
    let events = vec![
        TrajectoryEvent {
            timestamp_ms: 1000,
            session_id: "test".to_owned(),
            event_kind: EventKind::Write {
                file_path: "src/lib.rs".to_owned(),
                content: "todo!(\"stub\")".to_owned(),
            },
            outcome: None,
        },
        TrajectoryEvent {
            timestamp_ms: 2000,
            session_id: "test".to_owned(),
            event_kind: EventKind::Stop {
                final_message: "the next step is yours to run".to_owned(),
            },
            outcome: None,
        },
    ];
    let result = super::eval_trajectory_score(&events);
    assert!(result < 0, "stub + deferral should result in negative score");
}

#[test]
fn eval_advisory_empty_events_returns_zero() {
    let events: Vec<TrajectoryEvent> = vec![];
    let result = super::eval_trajectory_score(&events);
    assert_eq!(result, 0, "empty events should score 0");
}
