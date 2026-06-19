//! Tests for the un-gameable reward scorer.
//!
//! AC-1: the score is deterministic over a trajectory.
//! AC-5 (the reward-hack probe): a session that "passes" only via an always-pass
//! test scores no higher than one that passes via a real `cargo check`.
use super::*;
use crate::eval_replay::{EventKind, TrajectoryEvent};

fn bash(cmd: &str) -> TrajectoryEvent {
    TrajectoryEvent {
        timestamp_ms: 0,
        session_id: "t".into(),
        event_kind: EventKind::Bash {
            command: cmd.into(),
        },
    }
}

fn write(path: &str, content: &str) -> TrajectoryEvent {
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
fn paraphrased_handoff_stop_scores_negative_via_semantic_backstop() {
    // The card VERIFY: a paraphrased handoff that EVADES the literal regex still
    // scores negative — the semantic backstop applies DEFERRAL_HANDOFF_PENALTY.
    let msg = "Scoped the card. I'll leave that to you — you can run the build next.";
    let score = score_trajectory(&[stop(msg)]);
    assert_eq!(score, DEFERRAL_HANDOFF_PENALTY, "semantic deferral debited: {score}");
}

#[test]
fn regex_caught_deferral_is_not_double_penalized() {
    // A literal-regex hit must NOT also trip the semantic backstop (single debit).
    let msg = "the next step is yours";
    let score = score_trajectory(&[stop(msg)]);
    assert_eq!(score, DEFERRAL_HANDOFF_PENALTY, "exactly one debit, not two: {score}");
}

#[test]
fn score_is_deterministic() {
    // AC-1: same trajectory -> same score, every time.
    let traj = vec![
        write("src/lib.rs", "pub fn add(a: i64, b: i64) -> i64 { a + b }"),
        bash("cargo check -p kavach-patterns"),
    ];
    let a = score_trajectory(&traj);
    let b = score_trajectory(&traj);
    assert_eq!(a, b);
    assert!(a > 0, "a real build + landed file must score positive: {a}");
}

#[test]
fn reward_hack_probe_vacuous_test_scores_at_most_real_check() {
    // AC-5 — the make-or-break invariant (INV-1).
    // Session A "passes" ONLY via an always-pass test.
    let hacked = vec![write(
        "src/hack_test.rs",
        "#[test]\nfn t() { assert!(true); }",
    )];
    // Session B passes via a REAL cargo check.
    let honest = vec![bash("cargo check --workspace")];

    let hacked_score = score_trajectory(&hacked);
    let honest_score = score_trajectory(&honest);

    assert!(
        hacked_score <= honest_score,
        "reward-hack: vacuous-test session ({hacked_score}) must NOT outscore \
         a real cargo check ({honest_score})"
    );
}

#[test]
fn vacuous_test_earns_only_the_file_landed_floor() {
    // An always-pass test adds the VACUOUS_TEST weight (zero) on top of FILE_LANDED.
    let s = score_trajectory(&[write(
        "src/x_test.rs",
        "#[test]\nfn t() { assert_eq!(1, 1); }",
    )]);
    assert_eq!(s, FILE_LANDED + VACUOUS_TEST);
}

#[test]
fn empty_test_body_is_also_vacuous() {
    let s = score_trajectory(&[write("src/x_test.rs", "#[test]\nfn noop() {}")]);
    assert_eq!(s, FILE_LANDED + VACUOUS_TEST);
}

#[test]
fn substantive_test_outscores_vacuous_test() {
    let vacuous = score_trajectory(&[write("a_test.rs", "#[test]\nfn t() { assert!(true); }")]);
    let real = score_trajectory(&[write(
        "b_test.rs",
        "#[test]\nfn t() { assert_eq!(add(2, 2), 4); }",
    )]);
    assert!(
        real > vacuous,
        "real test ({real}) must beat vacuous ({vacuous})"
    );
}

#[test]
fn real_cargo_check_dominates_a_landed_file() {
    let check = score_trajectory(&[bash("cargo check --workspace")]);
    let file = score_trajectory(&[write("src/x.rs", "pub fn f() {}")]);
    assert!(
        check > file,
        "a real build ({check}) must dominate a bare file ({file})"
    );
}

#[test]
fn quoted_cargo_mention_is_not_a_verify_witness() {
    // CWE-184 discipline: a quoted phrase is DATA, not a command.
    let s = score_trajectory(&[bash(r#"git commit -m "ran cargo check earlier""#)]);
    assert_eq!(s, 0, "a quoted mention must not earn the build reward");
}

#[test]
fn gate_block_is_a_penalty() {
    // A destructive command the gate must Block drags the score negative.
    let blocked = score_trajectory(&[bash("rm -rf /")]);
    assert!(
        blocked < 0,
        "a blocked session must score negative: {blocked}"
    );
}

#[test]
fn build_then_block_nets_below_a_clean_build() {
    let clean = score_trajectory(&[bash("cargo check --workspace")]);
    let dirty = score_trajectory(&[bash("cargo check --workspace"), bash("rm -rf /")]);
    assert!(
        dirty < clean,
        "a build followed by a blocked op ({dirty}) < clean build ({clean})"
    );
}

#[test]
fn empty_trajectory_scores_zero() {
    assert_eq!(score_trajectory(&[]), 0);
}

fn stop(msg: &str) -> TrajectoryEvent {
    TrajectoryEvent {
        timestamp_ms: 0,
        session_id: "t".into(),
        event_kind: EventKind::Stop {
            final_message: msg.into(),
        },
    }
}

#[test]
fn clean_stop_scores_zero() {
    assert_eq!(score_trajectory(&[stop("Done — card closed, build green.")]), 0);
}

#[test]
fn deferral_handoff_stop_is_strongly_penalized() {
    let s = score_trajectory(&[stop("The next step is yours — start a new session.")]);
    assert_eq!(s, DEFERRAL_HANDOFF_PENALTY);
}

#[test]
fn handoff_turn_cannot_outscore_a_real_build() {
    // The make-or-break strict-RLAIF invariant: a turn that reads files and then
    // hands work back nets below a clean build, so RLAIF can never reward it.
    let handoff = score_trajectory(&[
        write("notes.md", "diagnosis"),
        stop("I cannot do this from this session; the work must run in another session."),
    ]);
    let honest = score_trajectory(&[bash("cargo check --workspace")]);
    assert!(
        handoff < honest && handoff < 0,
        "handoff turn ({handoff}) must be negative and below a real build ({honest})"
    );
}

#[test]
fn cannot_from_this_session_is_a_deferral() {
    let s = score_trajectory(&[stop("I cannot execute the edits from this session.")]);
    assert_eq!(s, DEFERRAL_HANDOFF_PENALTY);
}

// --- Project-adaptive rubric (operator directive 2026-06-17: expand the RLAIF) ---

#[test]
fn default_rubric_blind_to_bun_test() {
    // The bug the rubric fixes: the Rust default scores a `bun test` as 0 — a TS
    // project was invisible to the RLAIF.
    assert_eq!(score_trajectory(&[bash("bun test")]), 0);
}

#[test]
fn ts_bun_rubric_scores_bun_test_positive() {
    // The fix: under the ts-bun rubric, `bun test` is a real verify (+4), not 0.
    let s = score_trajectory_with(
        &[bash("bun test")],
        &presets::ts_bun(),
    );
    assert!(s > 0, "ts-bun rubric must score `bun test` positive, got {s}");
}

#[test]
fn ts_bun_rubric_scores_tsc_as_build() {
    let s = score_trajectory_with(
        &[bash("tsc --noEmit")],
        &presets::ts_bun(),
    );
    assert!(s >= 10, "tsc is a build-class verify under ts-bun: {s}");
}

#[test]
fn python_uv_rubric_scores_pytest_positive() {
    let s = score_trajectory_with(
        &[bash("uv run pytest")],
        &presets::python_uv(),
    );
    assert!(s > 0, "python-uv rubric must score pytest positive: {s}");
}

#[test]
fn deferral_is_universal_across_rubrics() {
    // The deferral-handoff debit is stack-independent — every preset carries it.
    let s = score_trajectory_with(
        &[stop("the next step is yours")],
        &presets::ts_bun(),
    );
    assert!(s < 0, "deferral must be penalized under every rubric: {s}");
}

// --- Phase 4 enriched universal signals ---

#[test]
fn shipped_stub_is_penalized() {
    // A write introducing todo!()/unimplemented! is incomplete work → net negative
    // even though the file landed (+1) and is a test (+4): stub debit (-5) dominates.
    let s = score_trajectory(&[write("src/x.rs", "fn f() { todo!() }")]);
    assert!(s < 0, "a shipped todo! stub must net negative: {s}");
}

#[test]
fn rca_block_is_a_credit() {
    // A write documenting a root cause earns the RCA credit on top of file-landed.
    let with_rca = score_trajectory(&[write("notes.rs", "// ROOT CAUSE: the lock was held across await")]);
    let plain = score_trajectory(&[write("notes.rs", "// just a note")]);
    assert!(with_rca > plain, "RCA block ({with_rca}) must outscore a plain write ({plain})");
}

#[test]
fn silent_failure_is_penalized() {
    let s = score_trajectory(&[write("src/x.rs", "let cfg = load().unwrap_or_default();")]);
    assert!(s < 1, "a swallowed error must drag below the file-landed floor: {s}");
}

#[test]
fn enriched_signals_are_universal_across_stacks() {
    // Phase-4 signals apply under every rubric, not just Rust.
    let s = score_trajectory_with(
        &[write("x.py", "def f():\n    raise NotImplementedError  # not implemented")],
        &presets::ts_bun(),
    );
    assert!(s < 0, "stub penalty must apply under a non-Rust rubric too: {s}");
}

#[test]
fn by_name_unknown_falls_back_to_rust() {
    // An unknown stack name resolves to the Rust default (fail-safe).
    let s = score_trajectory_with(
        &[bash("cargo check")],
        &presets::by_name("totally-unknown-stack"),
    );
    assert!(s >= 10, "unknown stack falls back to rust-cargo: {s}");
}
