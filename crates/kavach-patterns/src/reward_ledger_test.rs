//! Tests for the balance-sheet ledger: net must equal the scalar reward, and the
//! itemization must split credits (verified work) from debits (mistakes).
use super::*;
use crate::eval_replay::{EventKind, TrajectoryEvent};

fn bash(cmd: &str) -> TrajectoryEvent {
    TrajectoryEvent {
        timestamp_ms: 0,
        session_id: "t".into(),
        event_kind: EventKind::Bash { command: cmd.into() },
        outcome: None,
    }
}

fn stop(msg: &str) -> TrajectoryEvent {
    TrajectoryEvent {
        timestamp_ms: 0,
        session_id: "t".into(),
        event_kind: EventKind::Stop { final_message: msg.into() },
        outcome: None,
    }
}

#[test]
fn net_equals_scalar_reward() {
    // The ledger net can NEVER disagree with score_trajectory — the invariant.
    let traj = vec![bash("cargo check --workspace"), bash("cargo nextest run")];
    let ledger = build_ledger(&traj);
    assert_eq!(ledger.net, score_trajectory(&traj));
}

#[test]
fn build_run_is_a_credit_line() {
    let ledger = build_ledger(&[bash("cargo check --workspace")]);
    assert!(ledger.credits.iter().any(|l| l.kind == "build" && l.weight > 0));
    assert!(ledger.debits.is_empty());
}

#[test]
fn deferral_stop_is_a_debit_line() {
    let ledger = build_ledger(&[stop("the next step is yours")]);
    assert!(ledger.debits.iter().any(|l| l.kind == "deferral_handoff" && l.weight < 0));
    assert!(ledger.credits.is_empty());
}

#[test]
fn clean_turn_has_no_debits() {
    let ledger = build_ledger(&[bash("cargo check"), stop("Done, verified.")]);
    assert!(ledger.debits.is_empty());
}
