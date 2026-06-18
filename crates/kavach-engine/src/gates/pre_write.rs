// hub: pre_write pipeline orchestrator — calls 4 stages in order.
// Stage 1: Security (path, secrets, python, memory)
// Stage 2: Enforcement (skills, research, evidence, tests, new crate)
// Stage 3: Guards (chain, rust, ts, sql, algo, platform P0 blocks)
// Stage 4: Advisory (RAG, P1 warnings, Karpathy, tailwind, algo inject)
//
// `prelude` holds the Stage 0 phase/iteration advisories; `advisory_ctx` builds
// the Stage 4 allow-time context block.
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

    // Stage 0: SDLC-phase + iteration-scope advisories (demoted-to-advisory nudges).
    if let Some(advisory) = prelude::check(&ctx, &session) {
        super::turn_relay::exit_pre_write_allow_relay(&mut session, Some(&advisory));
        return Ok(());
    }

    // Stage 1: Security
    match super::pre_write_security::check(&ctx) {
        SecurityResult::Block(reason) => {
            drop(kavach_hook::exit_pre_tool_deny(&reason));
            return Ok(());
        }
        SecurityResult::AllowEarly(warn) => {
            super::turn_relay::exit_pre_write_allow_relay(&mut session, Some(&warn));
            return Ok(());
        }
        SecurityResult::Pass => {}
    }

    // Stage 2: Enforcement — DEMOTED to advisory per roadmap.unit.gate-severity-classification.
    // Routed through router::emit; the per-call budget resets on each new tool_use_id
    // and the per-turn budget caps total fires at 10 to prevent the "stack of blocks"
    // anti-pattern. SOURCE: roadmap.unit.gate-severity-router.
    super::router::observe_tool_call(&mut session, &input.tool_use_id);
    if let Some(advisory) = super::pre_write_enforcement::check(&ctx, input, &mut session) {
        super::router::emit(
            &mut session,
            kavach_hook::GateSeverity::P2Advise,
            "pre_write_enforcement",
            &advisory,
        );
        return Ok(());
    }

    // Stage 3: Language guards (P0 blocks)
    let guard_result = super::pre_write_guards::check(&ctx, input, &session);
    if let Some(block) = guard_result.block {
        drop(kavach_hook::exit_pre_tool_deny(&block));
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
    let context = advisory_ctx::build(&ctx, input, &mut session, &guard_result);

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
