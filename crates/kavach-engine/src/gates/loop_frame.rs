//! `[LOOP]` and `[REWARD]` frame formatters + compact turn shadow builder.
//! SOURCE: docs/loop-engineering-injection-strategy.md Part IV.
use std::fmt::Write as _;

use kavach_chain::IntentAnalysis;
use kavach_session::SessionState;

/// Byte cap for the per-turn shadow relayed on Cursor preToolUse.
pub(crate) const TURN_SHADOW_CAP: usize = 800;
/// Byte cap for session-start `[REWARD:stats]`.
pub(crate) const REWARD_SESSION_CAP: usize = 400;
/// Rolling 30-day baseline pass rate (fail-soft when RPC stats unavailable).
const BASELINE_30D: f64 = 0.61;

/// Build the compact turn shadow persisted on `UserPromptSubmit` and flushed on Cursor.
#[must_use]
pub(crate) fn build_turn_shadow(
    session: &SessionState,
    intent: &IntentAnalysis,
    harness_pattern: &str,
    top_skill: Option<&str>,
) -> String {
    let mut out = String::with_capacity(TURN_SHADOW_CAP);
    writeln!(
        out,
        "[INTENT] type:{} risk:{} complexity:{}",
        intent.intent_type, intent.risk_level, intent.complexity
    )
    .ok();
    writeln!(out, "[HARNESS] {harness_pattern}").ok();
    let phase = if session.current_phase.is_empty() {
        "PLAN"
    } else {
        session.current_phase.as_str()
    };
    let card = if session.current_kanban_card.is_empty() {
        "(none)"
    } else {
        session.current_kanban_card.as_str()
    };
    let iteration = if session.current_iteration_file.is_empty() {
        "(none)"
    } else {
        session.current_iteration_file.as_str()
    };
    writeln!(
        out,
        "[PHASE] phase:{phase} card:{card} iteration:{iteration}"
    )
    .ok();
    if let Some(skill) = top_skill {
        writeln!(out, "[RAG:skill] {skill}").ok();
    }
    writeln!(out, "[HARNESS_ENV] cursor:native").ok();
    let loop_compact = build_loop_compact(session, None);
    if !loop_compact.is_empty() {
        out.push('\n');
        out.push_str(&loop_compact);
    }
    truncate_utf8(&out, TURN_SHADOW_CAP)
}

/// Full `[LOOP]` frame for stop followup (prepend to `[AUTO_CONTINUE]`).
#[must_use]
pub(crate) fn build_loop_stop(session: &SessionState, goal_title: Option<&str>) -> String {
    let title = goal_title
        .or_else(|| resolve_goal_title(session))
        .unwrap_or("(none)");
    let harness = loop_harness_label(session);
    let iter = session.loop_iteration.max(session.turn_count);
    format!(
        "[LOOP] — DO NOT END THIS TURN. Execute the dispatched card now.\n\
         goal: {title}\n\
         harness: {harness}\n\
         iteration: {iter}\n\
         terminate ONLY on 3-witness (rg ∧ diff ∧ build)\n\
         on done: status-update + dispatch the next card THIS SAME turn\n\n"
    )
}

/// Three-line compact `[LOOP]` for `pre_write` production paths.
#[must_use]
pub(crate) fn build_loop_compact(session: &SessionState, goal_title: Option<&str>) -> String {
    let title = goal_title
        .or_else(|| resolve_goal_title(session))
        .unwrap_or("(none)");
    let harness = loop_harness_label(session);
    let iter = session.loop_iteration.max(session.turn_count);
    format!(
        "[LOOP] goal:{title} harness:{harness} iter:{iter} \
         done:3-witness→close→next same turn"
    )
}

/// Session-start running reward stats (~400 bytes, optional budget).
#[must_use]
pub(crate) fn build_reward_session_stats(session: &SessionState) -> Option<String> {
    if session.reward_session_total == 0 && session.last_reward_summary.is_empty() {
        return None;
    }
    let total = i32::max(session.reward_session_total, 1);
    let pass = session.reward_session_pass;
    #[expect(
        clippy::float_arithmetic,
        reason = "GRPO-style advantage needs a fractional pass rate vs baseline"
    )]
    let rate = f64::from(pass) / f64::from(total);
    #[expect(
        clippy::float_arithmetic,
        reason = "GRPO-style advantage is the delta vs the 30d baseline"
    )]
    let advantage = rate - BASELINE_30D;
    let exploit = if advantage >= 0.0 {
        "exploit"
    } else {
        "explore"
    };
    let mut out = format!(
        "[REWARD:stats]\n\
         session_pass_rate: {rate:.2} ({pass}/{total})\n\
         baseline_30d: {BASELINE_30D:.2}\n\
         advantage: {advantage:+.2} vs baseline — {exploit}\n"
    );
    if !session.last_reward_summary.is_empty() {
        out.push_str(&session.last_reward_summary);
        out.push('\n');
    }
    Some(truncate_utf8(&out, REWARD_SESSION_CAP))
}

/// Stop followup last-action reward line.
#[must_use]
pub(crate) fn build_reward_stop_last(session: &SessionState) -> String {
    if session.last_reward_summary.is_empty() {
        return String::new();
    }
    let total = i32::max(session.reward_session_total, 1);
    #[expect(
        clippy::float_arithmetic,
        reason = "GRPO-style advantage needs a fractional pass rate vs baseline"
    )]
    let rate = f64::from(session.reward_session_pass) / f64::from(total);
    #[expect(
        clippy::float_arithmetic,
        reason = "GRPO-style advantage is the delta vs the 30d baseline"
    )]
    let advantage = rate - BASELINE_30D;
    format!(
        "[REWARD:last] {}\n\
         advantage: {advantage:+.2} above baseline — exploit\n\n",
        session.last_reward_summary
    )
}

const fn resolve_goal_title(session: &SessionState) -> Option<&str> {
    if !session.current_kanban_card.is_empty() {
        return Some(session.current_kanban_card.as_str());
    }
    None
}

const fn loop_harness_label(session: &SessionState) -> &str {
    if !session.loop_target.is_empty() {
        return session.loop_target.as_str();
    }
    "loop-until-done"
}

fn truncate_utf8(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_owned();
    }
    let mut out = String::with_capacity(max);
    let mut used = 0usize;
    for ch in s.chars() {
        let next = used.saturating_add(ch.len_utf8());
        if next > max {
            break;
        }
        out.push(ch);
        used = next;
    }
    out
}

#[cfg(test)]
#[path = "loop_frame_test.rs"]
mod tests;
