// pre_write pipeline orchestrator: security → enforcement → guards → advisory.
mod advisory_ctx;
mod prelude;
mod skill_match;

use kavach_types::HookInput;

use crate::error::EngineError;
use crate::gates::pre_write_context::WriteContext;
use crate::gates::pre_write_security::SecurityResult;

/// Pre-write pipeline: security → enforcement → guards → advisory → approve.
///
/// ARCH: `PhaseGatedPreWrite` — gates activate based on current SDLC phase
/// PATTERN: `phase_gate` | SCOPE: `pre_write` | CAP: AP | SEARCHED: 2026-04
#[expect(
    clippy::unnecessary_wraps,
    reason = "gate orchestrator signature; all paths return Ok(()) by design"
)]
pub(crate) fn run(input: &HookInput) -> Result<(), EngineError> {
    let ctx = WriteContext::extract(input);
    let mut session = kavach_session::get_or_create_session();

    let comment_noise =
        kavach_patterns::comment_noise_guard::advise(ctx.file_path, &ctx.effective_content);

    // Stage 1: Security FIRST — a later advisory must not suppress a P0 block.
    match super::pre_write_security::check(&ctx) {
        SecurityResult::Block(reason) => {
            drop(kavach_hook::exit_pre_tool_deny(&reason));
            return Ok(());
        }
        SecurityResult::AllowEarly(warn) => {
            let merged = match &comment_noise {
                Some(n) => format!("{warn}\n\n{n}"),
                None => warn,
            };
            super::turn_relay::exit_pre_write_allow_relay(&mut session, Some(&merged));
            return Ok(());
        }
        SecurityResult::Pass => {}
    }

    // Stage 2: Enforcement carried forward, not early-returned — a skill nudge must not suppress Stage-4 violations. decision.gate.enforcement-merges-not-suppresses
    super::router::observe_tool_call(&mut session, &input.tool_use_id);
    let enforcement = super::pre_write_enforcement::check(&ctx, input, &mut session);

    // Stage 3: Language guards (P0 blocks)
    let guard_result = super::pre_write_guards::check(&ctx, input, &mut session);
    if let Some(block) = guard_result.block {
        drop(kavach_hook::exit_pre_tool_deny(&block));
        return Ok(());
    }

    // Stage 0: SDLC-phase + iteration-scope advisory, consulted only AFTER every P0 block stage (security + guards) so it can never suppress one. decision.gate.security-before-phase-advisory
    if let Some(advisory) = prelude::check(&ctx, &session) {
        let merged = match &comment_noise {
            Some(n) => format!("{advisory}\n\n{n}"),
            None => advisory,
        };
        super::turn_relay::exit_pre_write_allow_relay(&mut session, Some(&merged));
        return Ok(());
    }

    // Track file modification
    if !ctx.file_path.is_empty() {
        session.add_file_modified(ctx.file_path);
        if !session
            .files_modified_this_turn
            .contains(&ctx.file_path.to_owned())
        {
            session
                .files_modified_this_turn
                .push(ctx.file_path.to_owned());
        }
    }

    // Stage 4: Advisory collection (includes P1 advisories from tiered guards).
    let mut context = advisory_ctx::build(&ctx, input, &mut session, &guard_result);
    if let Some(advisory) = enforcement {
        super::router::emit(
            &mut session,
            kavach_hook::GateSeverity::P2Advise,
            "pre_write_enforcement",
            &advisory,
        );
        context = format!("{advisory}\n\n{context}");
    }
    if let Some(n) = comment_noise {
        context.push_str("\n\n");
        context.push_str(&n);
    }

    super::event_log::log_gate_decision(
        &session.session_id,
        "pre_write",
        "allow",
        ctx.file_path,
        &session.project,
    );
    super::turn_relay::exit_pre_write_allow_relay(&mut session, Some(&context));
    Ok(())
}
