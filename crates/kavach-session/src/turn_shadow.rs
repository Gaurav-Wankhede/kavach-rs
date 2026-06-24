//! Turn shadow + post-tool advisory queue for Cursor relay (Phases 1 & 5).
use crate::state::SessionState;

/// The realized verify outcome for a card, on the `[REWARD:last]` followup.
///
/// Four states, NOT a bool: the mechanical 3-witness oracle yields `Passed`
/// (+1) / `Failed` (-1); a genuine no-signal turn yields `Abstain` (0, not a
/// graded sample); and — RLAIF (Reinforcement Learning from AI Feedback, Bai
/// et al. 2022) — when the mechanical oracle would otherwise abstain, an
/// AUTONOMOUS AI judgment supplies `AiJudged(good)` so the bandit keeps
/// learning off AI feedback (a graded ±1 sample) where the mechanical signal
/// is blind. SOURCE: kavach `decision.arch.harness-rl.design-2026-06-05`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RewardOutcome {
    /// A verified-clean receipt landed (+1, mechanical 3-witness).
    Passed,
    /// A PROVEN failure (-1, mechanical 3-witness).
    Failed,
    /// RLAIF: no mechanical receipt, but the AI judged the action's observed
    /// effect — `true` = net advance (+1), `false` = net regression (-1).
    AiJudged(bool),
    /// No verification signal at all — neither +1 nor -1, does NOT count toward
    /// the session total (the false-negative-reward fix, preserved).
    Abstain,
}

impl RewardOutcome {
    /// `Some(true/false)` for a graded ±1 sample (mechanical OR AI-judged),
    /// `None` for a true abstention. The single place the four states collapse
    /// to the bandit's pass/total accounting so RLAIF and mechanical samples are
    /// counted identically.
    #[must_use]
    pub const fn graded(self) -> Option<bool> {
        match self {
            Self::Passed | Self::AiJudged(true) => Some(true),
            Self::Failed | Self::AiJudged(false) => Some(false),
            Self::Abstain => None,
        }
    }

    /// The human-readable `[REWARD:last]` tag for this outcome.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Passed => "PASSED (+1.0)",
            Self::Failed => "FAILED (-1.0)",
            Self::AiJudged(true) => "AI_JUDGED good (+1.0, RLAIF)",
            Self::AiJudged(false) => "AI_JUDGED bad (-1.0, RLAIF)",
            Self::Abstain => "ABSTAINED (no verification signal; 0.0, not penalized)",
        }
    }
}

/// Max bytes for the compact per-turn shadow (never compete with `[AUTONOMY_CONTRACT]`).
const TURN_SHADOW_CAP: usize = 800;
/// Max FIFO advisories queued between flushes.
const PENDING_ADVISORY_CAP: usize = 3;
/// Max bytes per queued advisory line.
const ADVISORY_LINE_CAP: usize = 200;

/// Which parts of the relay queue to flush on this hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RelayFlush {
    /// Post-tool advisories only — keep `[INTENT]`/`[LOOP]` shadow for pre-write.
    AdvisoriesOnly,
    /// Turn shadow + advisories — pre-write (point-of-action).
    Full,
}

impl SessionState {
    /// Persist a compact turn shadow and mark it pending for the next relay flush.
    pub fn store_turn_shadow(&mut self, shadow: &str) {
        self.turn_shadow = truncate_utf8(shadow, TURN_SHADOW_CAP);
        self.turn_shadow_pending = !self.turn_shadow.is_empty();
        self.save_or_log();
    }

    #[must_use]
    pub const fn turn_shadow_pending(&self) -> bool {
        self.turn_shadow_pending
    }

    /// Append lifecycle-hook context (preCompact, subagentStart) for the next
    /// Cursor relay flush. Merges into `turn_shadow` without displacing an
    /// existing shadow from intent — merge point: `take_relay_payload` via
    /// `kavach_engine::gates::turn_relay::merge_relay` on preToolUse/preWrite.
    pub fn queue_lifecycle_relay(&mut self, block: &str) {
        let trimmed = block.trim();
        if trimmed.is_empty() {
            return;
        }
        if self.turn_shadow.is_empty() {
            self.store_turn_shadow(trimmed);
        } else {
            let merged = format!("{}\n\n{}", self.turn_shadow, trimmed);
            self.store_turn_shadow(&merged);
        }
    }

    /// Queue a one-line post-tool advisory (FIFO, max 3).
    pub fn queue_pending_advisory(&mut self, line: &str) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return;
        }
        let entry = truncate_utf8(trimmed, ADVISORY_LINE_CAP);
        if self.pending_advisories.len() >= PENDING_ADVISORY_CAP {
            self.pending_advisories.remove(0);
        }
        self.pending_advisories.push(entry);
        self.save_or_log();
    }

    /// Drain the pending advisories as standalone lines, clearing them.
    ///
    /// Harness-NEUTRAL counterpart to `take_relay_payload`: that path is the
    /// Cursor relay (wraps in `[POST_TOOL_RELAY]`, merged only when the vendor is
    /// Cursor). This one is for the `UserPromptSubmit`/intent injector that runs on
    /// EVERY harness — it is what carries a stop-gate advisory (e.g. an
    /// un-interrogated loophole) forward into the NEXT turn's pre-implementation
    /// context instead of letting it die as stale prose. Returns `None` when empty.
    #[must_use]
    pub fn drain_pending_advisories(&mut self) -> Option<Vec<String>> {
        if self.pending_advisories.is_empty() {
            return None;
        }
        let drained = std::mem::take(&mut self.pending_advisories);
        self.save_or_log();
        Some(drained)
    }

    /// Take merged relay payload and clear the flushed parts.
    #[must_use]
    pub fn take_relay_payload(&mut self, flush: RelayFlush) -> Option<String> {
        let include_shadow = flush == RelayFlush::Full;
        let has_shadow =
            include_shadow && self.turn_shadow_pending && !self.turn_shadow.is_empty();
        let has_adv = !self.pending_advisories.is_empty();
        if !has_shadow && !has_adv {
            return None;
        }
        let mut out = String::new();
        if has_shadow {
            out.push_str(&self.turn_shadow);
            self.turn_shadow_pending = false;
        }
        if has_adv {
            if out.is_empty() {
                out.push_str("[POST_TOOL_RELAY]\n");
            } else {
                out.push_str("\n\n[POST_TOOL_RELAY]\n");
            }
            for (i, adv) in self.pending_advisories.iter().enumerate() {
                if i > 0 {
                    out.push('\n');
                }
                out.push_str(adv);
            }
            self.pending_advisories.clear();
        }
        self.save_or_log();
        Some(out)
    }

    /// Record verify outcome for `[REWARD:last]` stop followup.
    ///
    /// Four-state [`RewardOutcome`], NOT a bool: `Passed` (+1) / `Failed` (-1)
    /// are the mechanical 3-witness oracle; `AiJudged(good)` is the RLAIF path
    /// that supplies a graded ±1 from an AUTONOMOUS AI judgment where the
    /// mechanical oracle is blind; `Abstain` is a true no-signal turn (neither
    /// +1 nor -1, does NOT count toward the session total). FIX [false-negative
    /// reward / L2]: the old `bool` conflated "no receipt" with "failed", so an
    /// out-of-band-verified card was scored -1.0 — `Abstain` keeps that fix while
    /// RLAIF converts the formerly-inert 0.0 into a learnable signal.
    pub fn record_reward_outcome(&mut self, card: &str, outcome: RewardOutcome) {
        let card = if card.is_empty() { "(card)" } else { card };
        self.last_reward_summary = format!("last_action: {card} → verify {}", outcome.tag());
        // Only a GRADED sample (mechanical or AI-judged) counts toward the total;
        // a true abstention must not inflate the total or depress the pass-rate.
        if let Some(passed) = outcome.graded() {
            self.reward_session_total = self.reward_session_total.saturating_add(1);
            if passed {
                self.reward_session_pass = self.reward_session_pass.saturating_add(1);
            }
        }
        self.save_or_log();
    }
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
#[path = "turn_shadow_test.rs"]
mod tests;
