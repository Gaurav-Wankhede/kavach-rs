//! Forced terminal: run 3-witness on done cards, then collect in-flight bypass
//! state. On witness failure or unverified done cards → re-block. On empty queue
//! after witnesses pass → emit the clean STOP.
use core::ops::ControlFlow;

use super::probe::next_dispatch;
use crate::gates::stop::shared::StopCtx;
use crate::gates::stop_dispatch::{SOURCE_DOWN_KEY, AutoVerify, auto_verify_done_cards, claim_card};

/// Collect what is being bypassed by a forced stop, for the terminal advisory.
fn bypass_info(ctx: &StopCtx<'_>) -> String {
    let mut bypassed = Vec::new();
    if ctx.session.has_recent_failure() {
        bypassed.push(format!(
            "unresolved failure: {} (turn {}, blocked {} times)",
            ctx.session.last_failure_tool,
            ctx.session.last_failure_turn,
            ctx.session.failure_block_count,
        ));
    }
    if ctx.session.active_subagents > 0 {
        bypassed.push(format!(
            "{} active subagent(s)",
            ctx.session.active_subagents
        ));
    }
    if ctx.session.has_task() && ctx.session.task_status == "in_progress" {
        bypassed.push(format!("task in progress: {}", ctx.session.current_task));
    }
    if bypassed.is_empty() {
        "none".into()
    } else {
        bypassed.join(", ")
    }
}

/// The forced-terminal tail. Always returns `Break` — it is the last branch.
pub(super) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    let info = bypass_info(ctx);
    if info != "none" {
        ctx.session
            .add_case_fact(&format!("forced stop with bypassed: {info}"));
    }
    ctx.session.clear_failure();

    // RUN WITNESSES BEFORE ANY TERMINAL EXIT: the 3-witness must execute before
    // the agent exits, even on a forced stop with exhausted reblock counter.
    // If done cards exist and witnesses fail → re-block for repair, never exit.
    let outcome = auto_verify_done_cards(&ctx.session.project);
    match outcome {
        AutoVerify::WitnessFailed => {
            // Done cards exist but witnesses failed — this is real AI repair work.
            // Command repair instead of exiting.
            drop(kavach_hook::exit_stop_block(
                "[FORCED_TERMINAL_WITNESS_FAILED] done cards exist but workspace \
                 witnesses failed (cargo check / clippy / nextest / diff). This is \
                 fixable AI work. Re-block to command KEYSTONE_REPAIR. The loop must \
                 repair the failure before any terminal exit."
            ));
            ctx.session.increment_stop_reblock();
            return ControlFlow::Break(());
        }
        AutoVerify::VerifyRpcDown => {
            // Work is PROVEN but the promote-to-verified DB write failed (daemon
            // down). Never exit treating a finished card as incomplete; surface the
            // outage so the operator restarts the daemon and lands the status update.
            drop(kavach_hook::exit_stop_block(
                "[VERIFY_RPC_DOWN] done cards passed witnesses but the DB promotion \
                 (done -> verified) FAILED — kavach RPC/daemon unreachable. The work is \
                 NOT incomplete; the status write did not land. RECOVER: start `kavach rpc`, \
                 then `kavach db status-update <key> --status verified --project <slug>`, then resume.",
            ));
            ctx.session.increment_stop_reblock();
            return ControlFlow::Break(());
        }
        AutoVerify::Unprovable => {
            // Work cannot be proven — surface the clear blocker.
            drop(kavach_hook::exit_stop_block(
                "BLOCKED: `done` cards exist but work CANNOT BE PROVEN. The project is \
                 not a Rust workspace (no Cargo.toml) and KAVACH_VERIFY_CMD is not set. \
                 Either:\n\
                 1. Set env var KAVACH_VERIFY_CMD to a shell command that verifies the work, then resume.\n\
                 2. Manually promote the cards (kavach db roadmap update <key> --status verified) if work is proven by external audit.\n\
                 The loop yields only to the user's `Esc`."
            ));
            return ControlFlow::Break(());
        }
        AutoVerify::Promoted(_) | AutoVerify::NothingDone => {
            // Witnesses passed (or no done cards to verify) — proceed to check for runnable work.
        }
    }

    // AUTHORITATIVE EMPTY-QUEUE PROBE (Ralph-loop law): re-run the dependency-aware
    // chain to find the next task. A non-empty queue with runnable work means
    // the stop must NOT exit — re-block and dispatch instead.
    if let Some((tier, key, title)) = next_dispatch(&ctx.session.project)
        && key != SOURCE_DOWN_KEY
    {
        let _claimed = claim_card(&ctx.session.project, &key);
        ctx.session.increment_stop_reblock();
        drop(kavach_hook::exit_stop_block(&format!(
            "[AUTO_CONTINUE] backlog NOT empty — a completion left runnable work \
             queued. NEXT {tier} [{key}]: {title}. CLAIMED — resume NOW. (The \
             re-block breaker bounds SPINNING, not a non-empty queue; the loop \
             continues while work remains.)"
        )));
        return ControlFlow::Break(());
    }

    // SPIN GUARD: if reblock counter is exhausted with zero progress, the current
    // card is live-locked. Name it explicitly instead of silently allowing the stop.
    if ctx.session.stop_reblock_count > kavach_session::SessionState::max_stop_reblocks()
        && !ctx.session.has_progress_since_last_stop()
    {
        drop(kavach_hook::exit_stop_block(
            "[LIVE_LOCKED_CARD] The reblock counter is exhausted with zero progress \
             since the last stop attempt. The current card is stuck in a live-lock \
             (not spinning on runnable work, but not progressing either). Honest \
             exits: CLOSE (3-witness verify the work) or DELETE (discard the card). \
             Do NOT stop — make progress or delete the card, then resume. If the \
             card is genuinely un-buildable, delete it (`kavach db delete --category \
             roadmap --key <key>`) and the loop will dispatch the next task."
        ));
        return ControlFlow::Break(());
    }

    // Terminal for THIS turn: backlog provably empty, witnesses passed (or no done
    // cards to verify), and reblock counter is within bounds or progress was made.
    // The loop does NOT self-terminate — it re-scans the DB across statuses for the
    // NEXT task and yields only to the user's `Esc`.
    ctx.session.clear_stop_reblock();
    let context = kavach_hook::context_block(
        "AUTO_CONTINUE",
        &[
            (
                "why",
                "the dispatch tiers found nothing runnable, and all workspace \
                 witnesses passed (or no done cards to verify). Neither ends the loop: \
                 the DB still holds roadmap units and decisions to execute.",
            ),
            (
                "action",
                "Do NOT stop. Re-scan the DB for the next task across statuses — \
                 `kavach db query --category roadmap` and `--category decision` for \
                 any todo/unexecuted row. RESEARCH MODE FIRST: WebSearch the current \
                 authoritative source to scope it — TABULA RASA = TRUTH, never trust \
                 training weights — then claim and START it THIS turn. When the DB \
                 holds nothing actionable, keep the loop open and yield to the user's \
                 `Esc`.",
            ),
            ("bypassed", &info),
        ],
    );
    drop(kavach_hook::exit_stop_context(&context));
    ControlFlow::Break(())
}
