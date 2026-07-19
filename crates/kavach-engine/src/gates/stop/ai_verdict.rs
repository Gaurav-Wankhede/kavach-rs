//! Autonomous completion verdict — WITNESS-DERIVED, never self-assessed.
//!
//! ROOT CAUSE THIS REWRITE FIXES (operator directive 2026-06-18): the prior RLAIF
//! labeler scored a ±1 reward by STRING-MATCHING the assistant's own end-of-turn
//! prose (`ADVANCE_SIGNALS` like "3-witness"/"all tests pass" vs `REGRESSION_
//! SIGNALS`). That is self-assessment-as-truth: an agent that merely WROTE
//! "all tests pass" earned a +1 whether or not anything built. The DB then learned
//! — and recorded — a CLAIM as if it were evidence. This is the exact
//! evidence-over-inference violation that let false "done" labels accumulate.
//!
//! The fix: the verdict comes from the MECHANICAL witness run (cargo
//! check+clippy+nextest+diff, or `KAVACH_VERIFY_CMD`), NEVER from the message
//! text. `Passed` => +1, `Failed`/`SpawnError` => -1, `Unprovable` => abstain
//! (`None`). No prose is inspected; an agent cannot talk its way to a reward.
//! SOURCE: operator directive 2026-06-18 (witnesses, not assumptions) ·
//! kavach `decision.arch.harness-rl.design-2026-06-05` (RLAIF intent preserved,
//! the AI-feedback signal is now the objective build outcome, not self-report).
#[cfg(test)]
#[path = "ai_verdict_test.rs"]
mod tests;
use crate::gates::stop_dispatch::verify::witness::{WitnessRun, run_workspace_witnesses};
/// PURE map from an objective witness outcome to a ±1/abstain verdict.
///
/// `Passed` => `Some(true)`; `Failed`/`SpawnError` => `Some(false)` (a Rust project
/// that won't build is a regression); `Unprovable` => `None` (non-Rust + no
/// `KAVACH_VERIFY_CMD` — abstain rather than fabricate a reward). Split out so it is
/// unit-testable WITHOUT spawning the minutes-long cargo witnesses (purity at the
/// boundary — the impure run lives in [`extract_ai_verdict`]).
#[must_use]
pub(super) const fn verdict_from_witness(run: WitnessRun) -> Option<bool> {
    match run {
        WitnessRun::Passed => Some(true),
        WitnessRun::Failed | WitnessRun::SpawnError => Some(false),
        WitnessRun::Unprovable => None,
    }
}
/// Derive the autonomous completion verdict from the OBJECTIVE workspace witnesses,
/// NOT from the assistant's prose.
///
/// The `_message` is accepted but DELIBERATELY UNUSED — the verdict is evidence-
/// bound, never prose-derived (the self-assessment hole this rewrite closed).
///
/// HOT-PATH GUARD: running the full workspace witnesses (cargo check+clippy+nextest)
/// costs minutes, so it MUST NOT fire on every stop. It runs ONLY when
/// `KAVACH_RLAIF_WITNESS=1` opts in (reward-training contexts); otherwise the
/// labeler ABSTAINS (`None`) — fail-safe: never fabricate a reward, never block the
/// stop gate on a build. The mechanical 3-witness receipt (auto-verify path) remains
/// the primary ground truth; this only fills the gap when explicitly enabled.
#[must_use]
pub(super) fn extract_ai_verdict(_message: &str) -> Option<bool> {
    if std::env::var("KAVACH_RLAIF_WITNESS").as_deref() != Ok("1") {
        return None;
    }
    // No card content in this RLAIF path → no per-card WITNESS_ROOT hint; the env
    // override + CWD discovery still apply inside run_workspace_witnesses.
    verdict_from_witness(run_workspace_witnesses(None, None))
}
