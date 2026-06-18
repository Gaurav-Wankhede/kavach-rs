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

mod advisory_detectors;
mod ai_verdict;
mod dispatch;
mod foreign_tree_logic;
mod inflight;
mod pattern_extract;
mod phase;
mod reward_backfill;
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
            loophole_advisory: None,
            shallow_advisory: None,
            continuation_advisory: None,
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
    // RLAIF autonomous labeler: when no mechanical 3-witness receipt landed,
    // derive an AI-feedback verdict from the assistant's own end-of-turn
    // self-assessment so the bandit still learns where the mechanical oracle is
    // blind. The mechanical receipt is ground truth and is never overridden — we
    // only fill the gap. No human input, no extra model call.
    if !session.goal_receipt_pass {
        session.ai_verdict = ai_verdict::extract_ai_verdict(&msg);
    }
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

    // Shallow-verdict guard (re-enforced from the advisory path, NOT a HALT — the
    // pure-HALT version was removed under the no-block policy and the detector was
    // left orphaned: `shallow_verdict_guard` was a `pub mod` with ZERO call sites.
    // A "clean / wired / no-defect / safe" verdict asserted without leaf-depth
    // evidence (`file.rs:NN` or an [RCA] block) is the shallow-research signature.
    // Same wiring as loophole/capture above: stash the advisory for clean-exit AND
    // record a mistake row at the computation site so the learning loop sees it on
    // every stop (the loop usually short-circuits before clean_exit).
    let shallow_advisory = kavach_patterns::shallow_verdict_guard::detect_shallow_verdict(&msg);
    if shallow_advisory.is_some() {
        drop(kavach_session::record_mistake(&kavach_session::Mistake {
            project: &session.project,
            gate: "shallow_verdict",
            banned_sample: "asserted a clean/wired/no-defect verdict with no file:line citation and no [RCA] block",
            correct_action: "open the entry->...->logic call path and cite the file:line you read, or drop the verdict",
            turn: session.turn_count,
        }));
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
    advisory_detectors::run(&mut session, &msg, wrote_this_turn);

    // Build the shared context once; guards thread it.
    let mut ctx = StopCtx {
        input,
        session: &mut session,
        semver_advisory: None,
        capture_advisory,
        loophole_advisory,
        shallow_advisory,
        continuation_advisory,
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
        // USER-FOCUS OVERRIDE (owner directive 2026-06-18): runs BEFORE the
        // dispatch chain. When the user steered THIS turn and no card is mid-work,
        // it allows a clean stop so the gate does NOT drag the session onto a
        // different queued card than the user's live instruction. On turns the user
        // did NOT just speak, it falls through and the autonomous dispatch chain
        // drives the loop exactly as before.
        phase::user_focus,
        // FOREIGN-DIRTY-TREE GUARD (Case B): allow-stop when `git status` shows the
        // shared checkout is dirty far beyond THIS session's own writes — another
        // live session is mid-edit, so dispatching an editing card would clobber it.
        // Falls through on a clean tree or own-only dirt (own git worktree => no-op).
        phase::foreign_tree,
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

/// Loophole self-interrogation: if the turn claimed completion on a risk-bearing
/// path WITHOUT a `Loopholes closed:` line, record a mistake-ledger row, queue
/// the next-turn pending advisory, surface concrete suspect sites, and return the
/// clean-exit ride-along advisory. Extracted from `run` to keep the orchestrator
/// under the 100-LOC ceiling.
///
/// PRECISION GUARD (false-positive fix): a loophole can only be LIVE if this turn
/// actually WROTE a risk-bearing path. `last_write_turn == turn_count` iff a file
/// was Written/Edited this turn (set by the `post_write` gate), so the message-text
/// trigger cannot fire on a read-only Q&A turn whose PROSE merely describes past
/// risk fixes.
///
/// Recording at the computation site (not in `clean_exit`) is deliberate: the loop
/// usually short-circuits at `dispatch::reblock` and never reaches `clean_exit`, so
/// this is the only place the learning loop sees the omission on every stop.
fn loophole_check(session: &mut kavach_session::SessionState, msg: &str) -> Option<String> {
    let wrote_this_turn = session.last_write_turn == session.turn_count;
    let advisory = super::loophole_guard::check_stop_interrogation(msg, wrote_this_turn)?;
    drop(kavach_session::record_mistake(&kavach_session::Mistake {
        project: &session.project,
        gate: "loophole_uninterrogated",
        banned_sample: "shipped risk-bearing work without CLOSING the loopholes (no Loopholes closed: line)",
        correct_action: "fix each of the 6 attack-lens loopholes at its root THIS turn (or file a card), then emit a Loopholes closed: line",
        turn: session.turn_count,
    }));
    // Queue it for the NEXT turn's intent injector to drain (intent/context.rs
    // [CARRY_FORWARD]). Call queue_pending_advisory DIRECTLY (not
    // turn_relay::queue_advisory): the latter is Cursor-gated via should_relay(),
    // so on Claude Code it would silently no-op and the loophole would vanish. The
    // intent-injector drain is harness-neutral, so the queue must be too.
    session.queue_pending_advisory("[LOOPHOLE] last turn shipped risk-bearing work without a `Loopholes closed:` line — a loophole may be LIVE. FIX FIRST: run the 6 attack lenses (concurrency/failure/malformed/authz/replay/boundary) and CLOSE each at its root this turn (or file a card), then emit `Loopholes closed:`. Do this BEFORE any new work — fixing beats documenting.");
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
    drop(kavach_session::record_mistake(&kavach_session::Mistake {
        project: &session.project,
        gate: "continuation_menu_question",
        banned_sample: "ended the turn on a 'continue or pause?' permission question while [AUTO_CONTINUE] already commanded autonomous continuation",
        correct_action: "do NOT ask to continue — the gate already dispatched the next move; START it this turn (or, on a genuinely drained board, STATE the clean stop without a question)",
        turn: session.turn_count,
    }));
    // Re-surface the omission at the TOP of the next turn (harness-neutral
    // pending queue, not the Cursor-gated turn_relay), so the model sees it
    // BEFORE its next message — the only place that breaks the ask-again habit.
    session.queue_pending_advisory(
        "[CONTINUATION_MENU] last turn ended on a 'continue or pause?' question while the loop \
         directive already commanded continuation. Do NOT ask to continue — check the kavach DB \
         (kanban + decision/roadmap) and START the next task THIS turn. Asking to continue is the \
         forbidden deferral (global CLAUDE.md §autonomous_loop / §act_not_narrate).",
    );
    Some(continuation_menu_advisory())
}

/// The clean-exit ride-along text for a continuation-menu stall: the final
/// message asked permission to continue while the loop already commanded it.
/// Imperative, fix-first — points the model back at the DB, not at the user.
fn continuation_menu_advisory() -> String {
    "[CONTINUATION_MENU] Your final message ended the turn on a 'continue or pause?' \
     permission question — but the loop directive (the [AUTO_CONTINUE]/[ALL_BLOCKED] \
     verdict in this same stop) ALREADY told you the next move. Asking the user for \
     permission to do what the gate ordered is the forbidden deferral (global CLAUDE.md \
     §autonomous_loop §4_continue_not_stop / §act_not_narrate). Do NOT ask: check the \
     kavach DB (kanban + `kavach db query --category decision`/`--category roadmap`), \
     claim the next task, and START it THIS turn. If the board is genuinely drained, \
     STATE the clean stop as a fact — never as a question."
        .to_owned()
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
