//! PRIORITY 1: dispatch the next kanban task.
use core::ops::ControlFlow;

mod envelope;
mod gate_strip;
use envelope::{EnvelopeCtx, dispatch_envelope};

use super::source_down;
use crate::gates::event_log::log_gate_decision;
use crate::gates::loop_frame;
use crate::gates::stop::shared::StopCtx;
use crate::gates::stop_dispatch::{
    SOURCE_DOWN_KEY, card_entry_status, claim_card, get_next_task_info, live_lease_holder,
    next_task_directive,
};

/// `Break` with an `[AUTO_CONTINUE]` envelope if a task is pending; `Continue`
/// (fall through to the next tier) when the task tier is empty.
pub(super) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    let Some((priority, title)) = get_next_task_info(&ctx.session.project) else {
        return ControlFlow::Continue(());
    };
    if priority == SOURCE_DOWN_KEY {
        return source_down::block("backlog");
    }
    let claimed = claim_card(&ctx.session.project, &priority);
    if !claimed {
        // CAS miss on an already-`in_progress` card is expected (idempotent re-stop)
        // — reblock so Cursor's initial `loop_count:0` stop still dispatches runnable
        // work instead of a silent `{}` clean exit. Only fall through on a lost
        // `todo` race (another session claimed it first).
        let is_in_progress =
            card_entry_status(&ctx.session.project, &priority).is_some_and(|s| s == "in_progress");
        // Lease-aware resume (closes concurrent double-resume): an in_progress card
        // is resumable by THIS session only if no OTHER session holds a live lease.
        // A live foreign lease => its holder is still working (heartbeat-renewed), so
        // resuming would double-run it — fall through instead. No holder / my own
        // holder / unobservable lease (fail-open) => safe to resume.
        let foreign_live_lease =
            live_lease_holder(&priority).is_some_and(|holder| holder != ctx.session.session_id);
        let resume = is_in_progress && !foreign_live_lease;
        if is_in_progress && foreign_live_lease {
            log_gate_decision(
                &ctx.session.session_id,
                "stop:resume_foreign_live",
                "continue",
                &format!("card={priority} held by a live foreign lease; not resuming"),
                &ctx.session.project,
            );
        }
        if !resume {
            log_gate_decision(
                &ctx.session.session_id,
                "stop:claim_lost",
                "continue",
                &format!("card={priority} taken by another session; falling through"),
                &ctx.session.project,
            );
            return ControlFlow::Continue(());
        }
        log_gate_decision(
            &ctx.session.session_id,
            "stop:resume_in_progress",
            "block",
            &format!("resume={priority}: {title}"),
            &ctx.session.project,
        );
    }
    // Verify DB state after claim RPC: prevent narrate-without-persist
    // contradiction when transport fails mid-write. See decision.engine.readback-verify.
    let persisted_in_progress =
        card_entry_status(&ctx.session.project, &priority).is_some_and(|s| s == "in_progress");
    if claimed && !persisted_in_progress {
        log_gate_decision(
            &ctx.session.session_id,
            "stop:claim_not_persisted",
            "block",
            &format!("claim={priority} reported won but DB read-back != in_progress"),
            &ctx.session.project,
        );
    }
    if ctx.session.current_kanban_card != priority {
        ctx.session.current_kanban_card.clone_from(&priority);
        ctx.session.save_or_log();
    }
    if ctx.session.loop_active && !ctx.session.loop_exceeded_max() {
        ctx.session.increment_loop();
    }
    if claimed {
        log_gate_decision(
            &ctx.session.session_id,
            "stop:kanban_pending",
            "block",
            &format!("next_task={priority}: {title}"),
            &ctx.session.project,
        );
    }
    let loop_prefix = loop_frame::build_loop_stop(ctx.session, Some(&title));
    let reward_prefix = loop_frame::build_reward_stop_last(ctx.session);
    // The envelope emits STATE + the project's DYNAMIC directive (DB row
    // `gate.dispatch_directive`) — no fixed procedure prose in the binary.
    let directive = next_task_directive(&ctx.session.project);
    drop(kavach_hook::exit_stop_block(&dispatch_envelope(
        &EnvelopeCtx {
            proj: &ctx.session.project,
            priority: &priority,
            title: &title,
            loop_prefix: &loop_prefix,
            reward_prefix: &reward_prefix,
            claimed,
            persisted_in_progress,
            directive: directive.as_deref(),
        },
    )));
    ControlFlow::Break(())
}
