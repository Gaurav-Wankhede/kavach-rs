//! Stop gate: HARD BLOCK premature stops; force diagnosis and task resumption.
//!
//! This file is the thin ORCHESTRATOR — it owns only per-invocation setup
//! (session load, token accounting, decision-block + trajectory capture) and
//! then runs the guard pipeline in a fixed order. Every guard is a
//! single-responsibility microservice under `gates/stop/<group>/<name>.rs`;
//! each returns `ControlFlow::Break` to short-circuit (a decision was emitted)
//! or `Continue` to fall through to the next. No decision logic lives here.

use core::ops::ControlFlow;

use kavach_types::HookInput;

use crate::error::EngineError;

mod dispatch;
mod inflight;
mod phase;
mod shared;
mod terminal;

use shared::StopCtx;

/// Stop gate orchestrator. Runs the ordered guard pipeline; the first guard to
/// `Break` has emitted the hook decision. Reaching the end is unreachable in
/// practice (the terminal `clean_exit` always Breaks), but falls back to a
/// silent exit for total safety.
///
/// # Errors
/// Returns `Ok(())` on every path; the `Result` is part of the uniform
/// `run_gate` dispatch contract (see `gate_runner::run_gate`), not a fallible
/// computation here.
#[expect(
    clippy::unnecessary_wraps,
    reason = "signature is fixed by the run_gate dispatch table: every gate handler returns Result<(), EngineError> so gate_runner can match arms uniformly"
)]
pub(crate) fn run(input: &HookInput) -> Result<(), EngineError> {
    // FIRST: yield (do NOT block) while async work is in flight — background
    // tasks, crons, or teammates. Blocking here recreates issue #55754. These
    // run before any session mutation since they short-circuit to a silent exit.
    let mut session = kavach_session::get_or_create_session();
    {
        let mut ctx = StopCtx {
            input,
            session: &mut session,
            semver_advisory: None,
            capture_advisory: None,
        };
        if inflight::background(&mut ctx).is_break() {
            return Ok(());
        }
        if inflight::teammate(&mut ctx).is_break() {
            return Ok(());
        }
        if inflight::bulk_sweep(&mut ctx).is_break() {
            return Ok(());
        }
    }

    // Per-invocation setup (housekeeping, never a decision):
    // feed real per-turn token spend into the budget accumulator.
    if let Some(spent) = super::token_usage::extract_latest_token_usage(&input.transcript_path) {
        session.record_token_spend(spent);
    }
    // Pre-trim the message once to prevent whitespace-only classifier bypass.
    let msg = input.last_assistant_message.trim().to_owned();
    // Auto-extract [RCA]/[DESIGN]/[CRATE_DECISION]/[ARCH] blocks to the DB.
    super::stop_decisions::scan_decision_blocks(&msg, &session.project, session.turn_count.into());
    // Trajectory emitter (best-effort; a JSONL error must NOT block the gate).
    emit_trajectory(&session, &msg);

    // U3 capture-finding advisory: if the final message settled a decision in
    // prose but no decision/research DB write happened this turn, stash a
    // non-blocking nudge for the clean-exit context. The bracket-block scanner
    // above only catches explicit [RCA]/[DESIGN] markers; this covers the common
    // prose-settled case so a finding is not LOST.
    let wrote_decision_this_turn = session.last_db_write_turn == session.turn_count;
    let capture_advisory = kavach_patterns::unpersisted_decision_guard::detect_unpersisted_decision(
        &msg,
        wrote_decision_this_turn,
    );
    // U4 self-improve feed: record the unpersisted-decision finding to the
    // mistake ledger HERE — before the guard pipeline runs — so it fires on
    // EVERY stop regardless of which terminal branch wins. Wiring it into
    // clean_exit alone was dark in practice: the autonomous loop almost always
    // short-circuits at dispatch::reblock (runnable kanban work) and never
    // reaches clean_exit, so the feed never fired (proven by execution: the
    // gate emitted AUTO_CONTINUE, not CAPTURE_FINDING). Recording at the
    // computation site restores the learning-loop data feed the deleted
    // behavioral HALT guards used to provide — from the advisory path, no HALT.
    if capture_advisory.is_some() {
        drop(kavach_session::record_mistake(&kavach_session::Mistake {
            project: &session.project,
            gate: "capture_finding_unpersisted",
            banned_sample: "settled a decision in prose without persisting it to the DB",
            correct_action: "write a decision row the same turn (sync_to_kavach_db)",
            turn: session.turn_count,
        }));
    }

    // Build the shared context once; guards thread it.
    let mut ctx = StopCtx {
        input,
        session: &mut session,
        semver_advisory: None,
        capture_advisory,
    };

    // Ordered guard pipeline. `?`-style short-circuit via ControlFlow: the first
    // Break has emitted the hook decision.
    //
    // POLICY ("kill blocking, keep auto-continue"): a Stop must NEVER be HALTED.
    // Only DISPATCH guards remain — they re-claim and re-dispatch the next kanban
    // card (`[AUTO_CONTINUE]`), which is the autonomous loop the user wants
    // preserved. Every pure-HALT guard (bounty CVE/unsafe/license/deps,
    // behavioral deferral/incomplete/permission nags, tool/subagent/task/aegis/
    // empty-test failure blocks, review-isolation, shallow-verdict, and the
    // iteration "in progress" halt) was REMOVED from the pipeline so the gate can
    // dispatch the next card or exit clean, but can no longer stop the loop dead.
    //
    // `phase::iteration` is retained ONLY for its stale-file auto-recovery side
    // effect (clearing a crashed session's carry-over); its halt arm is disabled
    // in that guard. The dispatch chain still drives the Ralph loop:
    // kanban_status/kanban_card claim the next card, retry/first_pass emit
    // `[AUTO_CONTINUE]`, and clean_exit is the only terminal stop.
    let pipeline: &[fn(&mut StopCtx<'_>) -> ControlFlow<()>] = &[
        phase::iteration,
        phase::kanban_status,
        phase::kanban_card,
        dispatch::retry,
        dispatch::first_pass,
        terminal::clean_exit,
    ];
    for guard in pipeline {
        if guard(&mut ctx).is_break() {
            return Ok(());
        }
    }

    // Unreachable: terminal::clean_exit always Breaks. Fail safe if reached.
    drop(kavach_hook::exit_silent());
    Ok(())
}

/// Append this Stop event to the session trajectory JSONL for offline replay.
/// Best-effort: every failure is swallowed because a JSONL write error must
/// never block the Stop hook (which carries security duties).
fn emit_trajectory(session: &kavach_session::SessionState, msg: &str) {
    if session.session_id.is_empty() {
        return;
    }
    let Ok(path) = kavach_patterns::eval_replay::default_trajectory_path(&session.session_id)
    else {
        return;
    };
    let timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX));
    let event = kavach_patterns::eval_replay::TrajectoryEvent {
        timestamp_ms,
        session_id: session.session_id.clone(),
        event_kind: kavach_patterns::eval_replay::EventKind::Stop {
            final_message: msg.to_owned(),
        },
    };
    drop(kavach_patterns::eval_replay::emit_to_jsonl(&path, &event));
}
