//! Reward-hacking audit (harness-rl Wave P5) — the cross-cutting defense from
//! design §4.
//!
//! A self-improving gate that learns to pass the 3-witness CHEAPLY is worse than
//! no learning at all (constraint C3), so promotion is guarded by two
//! independent, fail-closed checks that live HERE, outside the controller it
//! audits:
//!
//! 1. The C2 SAFETY FLOOR (`safety_floor_held`): a structural proof that the
//!    learned action can only TIGHTEN a hard rule decision, never relax it. No
//!    learned policy may downgrade a `Block` to `Ask`/`Allow`. This is a static
//!    clamp on the action lattice, not a statistic — it holds regardless of data.
//!
//! 2. The TWO-TIER DRIFT MONITOR (`detect_reward_hacking`): a HARD reward (the
//!    code-checkable 3-witness) vs a SOFT held-out reward (a periodic REAL
//!    re-verification). If the soft signal is materially BELOW the hard one, the
//!    policy is passing the cheap witness without earning the real outcome —
//!    reward hacking — so the audit freezes promotion and raises an alarm.
//!
//! Both default to the conservative verdict on missing/non-informative data.
use crate::estimate::Estimate;
use crate::sample::Action;
#[cfg(test)]
#[path = "audit_test.rs"]
#[path = "audit_test.rs"]
mod tests;
/// Conservatism rank of an action on the safety lattice: `Block` (2) is the
/// tightest, then `Ask` (1), then `Allow` (0). A learned action is a RELAXATION
/// of a rule action iff its rank is strictly lower.
const fn conservatism(action: Action) -> u8 {
    match action {
        Action::Block => 2,
        Action::Ask => 1,
        Action::Allow => 0,
    }
}
/// The C2 safety-floor proof for one decision: does the learned `shadow` action
/// honor the floor set by the static `rule` action?
///
/// The floor is one-directional. When the rule is a hard `Block` (a P0/forbid
/// verdict), the learned policy may NOT weaken it — it must also be at least as
/// conservative. For a non-`Block` rule (an advisory `Allow`/`Ask` the
/// controller is allowed to tune), the learned action is free to move either
/// way: tuning advisory routing is the whole point of Layer C.
///
/// Returns `true` when the floor holds, `false` when the shadow RELAXES a hard
/// rule block — the single condition that must never reach production.
#[must_use]
pub const fn safety_floor_held(rule: Action, shadow: Action) -> bool {
    match rule {
        // A hard block is the safety floor: the learned policy may only match or
        // exceed its conservatism, never fall below it.
        Action::Block => conservatism(shadow) >= conservatism(Action::Block),
        // Advisory rule verdicts are the controller's tuning surface — any
        // learned action is permitted (the canary + OPE-CI gate it separately).
        Action::Allow | Action::Ask => true,
    }
}
/// Audit a batch of (rule, shadow) decision pairs against the C2 floor.
///
/// Returns the first VIOLATING pair (rule, shadow) if any learned action relaxed
/// a hard rule block, else `None`. A non-empty result is a release blocker: the
/// controller has proposed weakening a safety verdict and must not be promoted.
#[must_use]
pub fn first_floor_violation(pairs: &[(Action, Action)]) -> Option<(Action, Action)> {
    pairs
        .iter()
        .copied()
        .find(|&(rule, shadow)| !safety_floor_held(rule, shadow))
}
/// The audit's verdict on whether the learned policy is reward-hacking.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum AuditVerdict {
    /// Soft (real) value tracks hard (witness) value within tolerance — the
    /// policy is earning the outcomes it claims. Promotion may proceed.
    Healthy,
    /// Soft value is materially below hard value: the policy passes the cheap
    /// 3-witness without the real outcome. Freeze promotion and alarm. Carries
    /// the observed gap `hard − soft` for the alert.
    Hacking {
        /// How far the real (soft) value fell below the witness (hard) value.
        gap: f64,
    },
    /// Not enough trustworthy data on either tier to judge. Fail-closed: treat
    /// as not-promotable until a real held-out signal exists.
    Inconclusive,
}
/// Two-tier reward-hacking detector (design §4): compare the HARD witness value
/// against the SOFT held-out real-verification value.
///
/// `hard` is the policy value measured by the cheap, code-checkable 3-witness;
/// `soft` is the value measured by a periodic REAL re-verification on a held-out
/// set. `tolerance` is how far soft may trail hard before it counts as hacking
/// (a small positive slack absorbs sampling noise).
///
/// Verdict:
/// - either estimate non-informative (infinite SE / zero samples) ⇒
///   `Inconclusive` (fail-closed — we cannot clear a policy we cannot measure);
/// - `hard − soft > tolerance` ⇒ `Hacking { gap }` (freeze + alarm);
/// - otherwise ⇒ `Healthy`.
///
/// Note we compare POINT values, not lower bounds: a real downward gap between
/// the two reward channels is the hacking signal, and shrinking it through the
/// CI would only make the detector more permissive — the wrong direction for a
/// fail-closed guard.
#[must_use]
pub fn detect_reward_hacking(hard: &Estimate, soft: &Estimate, tolerance: f64) -> AuditVerdict {
    if !is_informative(hard) || !is_informative(soft) {
        return AuditVerdict::Inconclusive;
    }
    let gap = hard.value - soft.value;
    if gap > tolerance {
        AuditVerdict::Hacking { gap }
    } else {
        AuditVerdict::Healthy
    }
}
/// Whether an estimate carries usable information: a finite SE computed from at
/// least one sample. A zero-sample or infinite-SE estimate is non-informative.
const fn is_informative(e: &Estimate) -> bool {
    e.n > 0 && e.std_error.is_finite()
}
