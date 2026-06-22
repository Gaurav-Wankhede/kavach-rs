// hub: thin orchestrator — per-invocation setup + the guard pipeline only; every
// guard + helper lives in a gates/stop/<group>/<name>.rs leaf. No decision logic.
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
use crate::gates::directive_cache::dyn_directive;

mod advisory_detectors;
mod ai_verdict;
mod disobedience;
mod dispatch;
mod done_gaming;
mod foreign_tree_logic;
mod inflight;
mod pattern_extract;
mod phase;
mod reward_backfill;
mod shared;
pub(crate) mod spool_writes;
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
        let mut ctx = StopCtx::new(input, &mut session);
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
    // RLAIF autonomous labeler: when no mechanical 3-witness receipt landed,
    // derive an AI-feedback verdict from the assistant's own end-of-turn
    // self-assessment so the bandit still learns where the mechanical oracle is
    // blind. The mechanical receipt is ground truth and is never overridden — we
    // only fill the gap. No human input, no extra model call.
    if !session.goal_receipt_pass {
        session.ai_verdict = ai_verdict::extract_ai_verdict(&msg);
    }
    // Drain-before-write: replay any learning write a prior failed Stop spooled,
    // BEFORE this turn's own fire-and-forget writes. Idempotent (drain removes the
    // spool file first); best-effort (a DB-still-down replay re-spools).
    spool_writes::drain_and_replay();
    // P3a reward back-fill: grade this session's logged bandit decisions against
    // its 3-witness verify outcome (or the RLAIF AI verdict above when the
    // mechanical oracle abstains). Fire-and-forget; never blocks the gate.
    reward_backfill::backfill_session_rewards(&mut session);
    pattern_extract::trigger_on_verify(&session);
    // P6: learn from the freshly-graded rewards — fire db.policy_improve so the
    // daemon promotes a learned advisory policy iff all three gates clear.
    reward_backfill::trigger_policy_improve(&session);

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
    // Record the unpersisted-decision finding at the computation site (before the
    // pipeline) so it fires on EVERY stop — clean_exit alone was dark (loop short-
    // circuits at dispatch::reblock). See decision.engine.stop-mistake-feed-site.
    if capture_advisory.is_some() {
        let turn = session.turn_count;
        drop(kavach_session::record_mistake_surfaced(
            &mut session,
            "capture_finding_unpersisted",
            "settled a decision in prose without persisting it to the DB",
            "persist it THIS turn with the kavach db CLI: kavach db write --new \
             --project <slug> --category decision --key <key> --title <title> --content <text>",
            turn,
        ));
        if let Some(ref adv) = capture_advisory {
            super::turn_relay::queue_advisory(&mut session, adv);
        }
    }

    // Loophole self-interrogation (extracted to `loophole_check` to keep this
    // orchestrator under the 100-LOC ceiling): if the turn claimed completion on a
    // risk-bearing path WITHOUT a `Loopholes closed:` line, it records a mistake
    // row + queues the next-turn advisory + surfaces concrete suspect sites, and
    // returns the clean-exit ride-along advisory.
    let loophole_advisory = loophole_check(&mut session, &msg);

    // Shallow-verdict guard, advisory path (NOT a HALT): a clean/wired/safe verdict
    // with no file:line or [RCA] evidence. Stash for clean-exit + record mistake at
    // computation site. See decision.engine.stop-shallow-verdict-advisory.
    let shallow_advisory = kavach_patterns::shallow_verdict_guard::detect_shallow_verdict(&msg);
    if shallow_advisory.is_some() {
        let turn = session.turn_count;
        drop(kavach_session::record_mistake_surfaced(
            &mut session,
            "shallow_verdict",
            "asserted a clean/wired/no-defect verdict with no file:line citation and no [RCA] block",
            "open the entry->...->logic call path and cite the file:line you read, or drop the verdict",
            turn,
        ));
    }

    // Continuation-menu guard: the final message ENDED THE TURN on a "continue
    // or pause?" permission question while THIS gate's own `[AUTO_CONTINUE]`
    // verdict already commands continuation. Extracted to a helper so this
    // orchestrator stays under the 100-LOC ceiling. See `continuation_menu_check`.
    let continuation_advisory = continuation_menu_check(&mut session, &msg);

    // U5 advisory-detector dispatch: run the table of previously-DEAD stop-signal
    // detectors (permission-seek, name-then-stop, verification-claim-without-proof)
    // over the final message. Each firing entry records a mistake row + queues a
    // next-turn pending advisory. ADVISORY only (no HALT). The verification-claim
    // entries are gated behind `wrote_this_turn` inside the table.
    let wrote_this_turn = session.last_write_turn == session.turn_count;
    let stall = advisory_detectors::run(&mut session, &msg, wrote_this_turn);

    // Build the shared context once; guards thread it.
    let mut ctx = StopCtx {
        input,
        session: &mut session,
        semver_advisory: None,
        capture_advisory,
        loophole_advisory,
        shallow_advisory,
        continuation_advisory,
        research_unsourced: stall.research_unsourced,
        disobedience_handback: stall.handback_or_menu,
        argued_with_user: stall.argued_with_user,
    };

    // Ordered guard pipeline; first ControlFlow::Break emits the hook decision.
    // POLICY "kill blocking, keep auto-continue": a Stop is NEVER HALTED — only
    // DISPATCH guards remain ([AUTO_CONTINUE]); all pure-HALT guards removed.
    // phase::iteration kept only for stale-file auto-recovery (halt arm disabled).
    // See decision.engine.stop-no-halt-dispatch-only.
    let pipeline: &[fn(&mut StopCtx<'_>) -> ControlFlow<()>] = &[
        phase::iteration,
        // Argue-not-obey teeth: refuse a stop whose turn dismissed a fired imperative
        // in prose without obeying it. See decision.engine.disobedience-guard.
        disobedience::check,
        // USER-FOCUS OVERRIDE: user steered + no card mid-work -> allow clean stop
        // (don't drag onto a queued card). See decision.engine.stop-pre-dispatch-overrides.
        phase::user_focus,
        // FOREIGN-DIRTY-TREE: allow-stop when the shared checkout is dirty beyond
        // own writes (another session mid-edit). decision.engine.stop-pre-dispatch-overrides.
        phase::foreign_tree,
        phase::kanban_status,
        // DONE-GAMING BLOCK: refuse a completion-narrating stop with runnable cards
        // + no code/DB mutation. decision.engine.stop-pre-dispatch-overrides.
        done_gaming::check,
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

/// Record + surface the loophole signal for a risk-bearing turn: ledger row,
/// next-turn carry-forward, concrete suspect sites, and the ride-along advisory.
/// Gated on `last_write_turn == turn_count` (real write this turn) to avoid
/// read-only prose-trigger FPs. Recorded here (not `clean_exit`) since the loop
/// usually short-circuits at `dispatch::reblock`.
fn loophole_check(session: &mut kavach_session::SessionState, msg: &str) -> Option<String> {
    let wrote_this_turn = session.last_write_turn == session.turn_count;
    let advisory = super::loophole_guard::check_stop_interrogation(msg, wrote_this_turn)?;
    let turn = session.turn_count;
    drop(kavach_session::record_mistake_surfaced(
        session,
        "loophole_surface_unrecorded",
        "shipped risk-bearing work; the lens scan surfaced the risk surface",
        "the lens scan records suspect sites + the native triage agent fixes them — awareness, not a narration demand",
        turn,
    ));
    // Queue for next turn's intent-injector drain via queue_pending_advisory
    // DIRECTLY (turn_relay is Cursor-gated, would no-op on CC). Tag + marker + lens
    // names are frozen. See decision.engine.stop-loophole-carry-forward.
    let loophole_body = dyn_directive(
        "stop.loophole-carry-forward",
        "last turn touched a risk-bearing path; the lens scan recorded the suspect sites below. The native triage agent resolves them — surfaced for awareness, no narration required.",
    );
    session.queue_pending_advisory(&format!("[LOOPHOLE] {loophole_body}"));
    // M4 TEETH: run the bounded lens DETECTOR over this turn's git-changed Rust
    // files and surface CONCRETE suspected sites (lens + file:line) — real targets,
    // not just a reminder. Bounded so the Stop path can't stall.
    let changed = super::loophole_guard::changed_rust_files();
    let file_refs: Vec<(&str, &str)> =
        changed.iter().map(|(p, c)| (p.as_str(), c.as_str())).collect();
    if let Some(sites) = super::loophole_guard::scan_changed_for_loopholes(&file_refs) {
        session.queue_pending_advisory(&sites);
    }
    Some(advisory)
}

/// Detect the continuation-menu stall in the final message and, when present,
/// record a mistake row + queue a next-turn pending advisory, returning the
/// clean-exit ride-along text. Extracted from `run` to keep the orchestrator
/// under the 100-LOC ceiling.
///
/// The `detect_continuation_menu` detector (kavach-chain) catches the exact
/// phrasing the user reported ("Want me to continue to a new card, or pause
/// here?"); its NEG arm exempts the legitimate asks (a genuine
/// credential/irreversible/ambiguity ask, or a turn discussing the stop gate
/// itself). This closed the "defined-but-never-enforced" loophole: the detector
/// existed in kavach-chain with ZERO engine call sites, so the continue-or-pause
/// question sailed past every Stop. ADVISORY, never a HALT — loop-safe (the next
/// turn continues or states a clean stop). A regex-compile `Err` fails SAFE to
/// "no advisory" (a dead detector must never false-fire; the patterns are proven
/// to compile by the chain's own test).
fn continuation_menu_check(
    session: &mut kavach_session::SessionState,
    msg: &str,
) -> Option<String> {
    let fired = kavach_chain::stop_signals::detect_continuation_menu(msg).unwrap_or(false);
    if !fired {
        return None;
    }
    let turn = session.turn_count;
    drop(kavach_session::record_mistake_surfaced(
        session,
        "continuation_menu_question",
        "ended the turn on a 'continue or pause?' permission question while [AUTO_CONTINUE] already commanded autonomous continuation",
        "do NOT ask to continue — the gate already dispatched the next move; START it this turn (or, on a genuinely drained board, STATE the clean stop without a question)",
        turn,
    ));
    // Re-surface the omission at the TOP of the next turn (harness-neutral
    // pending queue, not the Cursor-gated turn_relay), so the model sees it
    // BEFORE its next message — the only place that breaks the ask-again habit.
    // Tag + §refs frozen; the deferral-correction imperative is research-refreshed.
    let menu_body = dyn_directive(
        "stop.continuation-menu-carry-forward",
        "last turn ended on a 'continue or pause?' question while the loop \
         directive already commanded continuation. Do NOT ask to continue — check the kavach DB \
         (kanban + decision/roadmap) and START the next task THIS turn. Asking to continue is the \
         forbidden deferral (global CLAUDE.md §autonomous_loop / §act_not_narrate).",
    );
    session.queue_pending_advisory(&format!("[CONTINUATION_MENU] {menu_body}"));
    Some(continuation_menu_advisory())
}

/// The clean-exit ride-along text for a continuation-menu stall: the final
/// message asked permission to continue while the loop already commanded it.
/// Imperative, fix-first — points the model back at the DB, not at the user.
fn continuation_menu_advisory() -> String {
    // [CONTINUATION_MENU] tag literal; the body (still naming the frozen
    // [AUTO_CONTINUE]/[BLOCKER_WALK] verdicts + §refs in its fallback) is research-refreshed.
    let body = dyn_directive(
        "stop.continuation-menu-ride-along",
        "Your final message ended the turn on a 'continue or pause?' \
         permission question — but the loop directive (the [AUTO_CONTINUE]/[BLOCKER_WALK] \
         verdict in this same stop) ALREADY told you the next move. Asking the user for \
         permission to do what the gate ordered is the forbidden deferral (global CLAUDE.md \
         §autonomous_loop §4_continue_not_stop / §act_not_narrate). Do NOT ask: check the \
         kavach DB (kanban + `kavach db query --category decision`/`--category roadmap`), \
         claim the next task, and START it THIS turn. If the board is genuinely drained, \
         STATE the clean stop as a fact — never as a question.",
    );
    format!("[CONTINUATION_MENU] {body}")
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
        // A Stop is a pure self-report claim — no objective outcome of its own.
        // The reward oracle scores it against the PRIOR Bash/build outcomes.
        outcome: None,
    };
    drop(kavach_patterns::eval_replay::emit_to_jsonl(&path, &event));
}
