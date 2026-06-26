//! Tests for the multidimensional ground-truth oracle: a completion claim is
//! contradicted only by a weighted quorum across orthogonal dimensions; agreement
//! or insufficient evidence leaves the self-report untouched; no single dimension
//! is decisive. SOURCE: decision.harness-reward-ground-truth-oracle.
use super::{DimVerdict, OracleConfig, oracle_penalty, oracle_penalty_with};
use super::{dim_output_failure, dim_process_exit};
use crate::eval_replay::{EventKind, EventOutcome, TrajectoryEvent};

fn ev(kind: EventKind, outcome: Option<EventOutcome>) -> TrajectoryEvent {
    TrajectoryEvent {
        timestamp_ms: 0,
        session_id: "t".into(),
        event_kind: kind,
        outcome,
    }
}

fn bash(cmd: &str, outcome: Option<EventOutcome>) -> TrajectoryEvent {
    ev(
        EventKind::Bash {
            command: cmd.into(),
        },
        outcome,
    )
}

fn stop(msg: &str) -> TrajectoryEvent {
    ev(
        EventKind::Stop {
            final_message: msg.into(),
        },
        None,
    )
}

#[test]
fn claimed_done_but_build_failed_is_dominantly_negative() {
    // Two orthogonal dimensions contradict: process-exit (Failure outcome) AND
    // output-vocab ("could not compile" in the command text is not present here,
    // so this rests on the exit dim) → quorum reached.
    let traj = vec![
        bash("cargo build", Some(EventOutcome::Failure)),
        stop("All done — fix is complete and the build is green."),
    ];
    assert_eq!(oracle_penalty(&traj), OracleConfig::default().penalty);
}

#[test]
fn output_vocab_alone_can_reach_quorum() {
    // Even WITHOUT a recorded Failure outcome, the output-failure-vocab dimension
    // ("could not compile") contradicts and the exit dim abstains — the agree side
    // is only the weak verify-ran/agree, so quorum still tips to contradiction.
    let traj = vec![
        bash(
            "cargo build 2>&1 | tee log  # could not compile",
            Some(EventOutcome::Success),
        ),
        stop("Done — green."),
    ];
    // exit=Agree(2) [Success], output=Contradict(2) → contradict 2 vs agree 2; with
    // margin 1, quorum is NOT reached — a single contradicting dim cannot override an
    // equal-weight agreeing one. Proves no single dimension is dictator.
    assert_eq!(oracle_penalty(&traj), 0);
}

#[test]
fn claimed_done_and_build_passed_no_penalty() {
    let traj = vec![
        bash("cargo build", Some(EventOutcome::Success)),
        stop("Done — build passes."),
    ];
    assert_eq!(oracle_penalty(&traj), 0);
}

#[test]
fn no_objective_signal_is_silence_not_contradiction() {
    // All dimensions abstain (no outcome, no command) → no contradiction fabricated.
    let traj = vec![bash("ls", None), stop("Done.")];
    assert_eq!(oracle_penalty(&traj), 0);
}

#[test]
fn failure_without_a_completion_claim_no_penalty() {
    let traj = vec![
        bash("cargo build", Some(EventOutcome::Failure)),
        stop("The build is still broken; investigating the type error."),
    ];
    assert_eq!(oracle_penalty(&traj), 0);
}

#[test]
fn a_single_failure_poisons_a_later_success() {
    let traj = vec![
        bash("cargo build", Some(EventOutcome::Failure)),
        bash("true", Some(EventOutcome::Success)),
        stop("Fixed and complete."),
    ];
    assert_eq!(oracle_penalty(&traj), OracleConfig::default().penalty);
}

#[test]
fn empty_trajectory_is_zero() {
    assert_eq!(oracle_penalty(&[]), 0);
}

// ── Dimension orthogonality: each reads a different artifact ──────────────────

#[test]
fn process_exit_dimension_reads_only_recorded_outcomes() {
    assert_eq!(
        dim_process_exit(&[bash("x", Some(EventOutcome::Failure))]),
        DimVerdict::Contradict
    );
    assert_eq!(
        dim_process_exit(&[bash("x", Some(EventOutcome::Success))]),
        DimVerdict::Agree
    );
    // No recorded outcome → abstain, never guess.
    assert_eq!(dim_process_exit(&[bash("x", None)]), DimVerdict::Abstain);
}

#[test]
fn output_dimension_reads_only_command_text_vocab() {
    let vocab = vec!["could not compile".to_owned()];
    assert_eq!(
        dim_output_failure(&[bash("cargo b # could not compile", None)], &vocab),
        DimVerdict::Contradict
    );
    // Absence of a failure token is NOT proof of success — abstain, not agree
    // (orthogonality: only the process-exit dim, a different artifact, may agree).
    assert_eq!(
        dim_output_failure(&[bash("cargo b", None)], &vocab),
        DimVerdict::Abstain
    );
    // No command at all → abstain.
    assert_eq!(
        dim_output_failure(&[stop("done")], &vocab),
        DimVerdict::Abstain
    );
}

#[test]
fn config_is_data_db_override_changes_the_verdict() {
    // The "hardcoded parameters" objection killed: the SAME trajectory flips from
    // no-penalty to penalty purely by swapping the config (as a DB row would).
    let traj = vec![
        bash(
            "cargo build 2>&1  # could not compile",
            Some(EventOutcome::Success),
        ),
        stop("Done — green."),
    ];
    // Default weights: exit=Agree(2), output=Contradict(2) → 2 vs 2, no quorum → 0.
    assert_eq!(oracle_penalty(&traj), 0);
    // A DB-tuned config that trusts the output vocab more flips the quorum.
    let strict = OracleConfig {
        w_output: 5,
        ..OracleConfig::default()
    };
    assert_eq!(oracle_penalty_with(&traj, &strict), strict.penalty);
}

#[test]
fn custom_failure_vocab_from_config_is_honored() {
    // Vocab is data: a runner that says "BUILD BROKE" (not in the default list)
    // is caught once the DB row adds that token — no source edit needed.
    let traj = vec![
        bash("./build.sh  # BUILD BROKE", Some(EventOutcome::Failure)),
        stop("Done."),
    ];
    let cfg = OracleConfig {
        failure_vocab: vec!["build broke".to_owned()],
        ..OracleConfig::default()
    };
    // exit=Contradict(2) + output=Contradict(2) vs agree 0 → quorum.
    assert_eq!(oracle_penalty_with(&traj, &cfg), cfg.penalty);
}
