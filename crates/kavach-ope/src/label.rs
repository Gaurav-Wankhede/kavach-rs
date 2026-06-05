//! Reward labeler — join a logged gate decision to its LATER 3-witness verify
//! outcome and produce the realized reward `r` for that `(x, a)` row.
//!
//! This is the back-fill the Layer-A emit deferred (it logged `reward = None`).
//! It is a PURE function of the action and the downstream outcome — the caller
//! fetches the un-rewarded `BanditRow`, finds its matching verify event, and
//! calls [`label`] to get the scalar to write back.
//!
//! INV reward-hacking guard (design C3/P5): the reward must score a FALSE BLOCK
//! — the gate blocked a change the dev overrode and it then verified clean — as
//! the costly error, the SAME as a false allow. Otherwise an over-firing gate
//! would look free and the optimizer would learn to block everything.

use crate::sample::Action;

/// What the 3-witness verify reported for the work the decision gated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum VerifyOutcome {
    /// The change went through (gate allowed, or was overridden) and the
    /// 3-witness passed: build + diff landed + tests green.
    VerifiedClean,
    /// The change went through and verify FAILED (build/test broke).
    VerifyFailed,
    /// The gate blocked and the dev did NOT override — the block stood. We never
    /// observe a counterfactual verify, so this is the abstention outcome.
    BlockedAndAccepted,
    /// The gate blocked but the dev OVERRODE it, and the change then verified
    /// clean — proof the block was a false positive.
    BlockedThenOverriddenClean,
}

/// The realized reward scalar for a logged decision, on the same scale the OPE
/// estimators consume (`+1` good, `0` neutral abstention, `-1` costly error).
///
/// Mapping (fail-closed bias — a false decision in EITHER direction is `-1`):
/// - Allow/Ask + `VerifiedClean`  -> `+1` (let good work through)
/// - Allow      + `VerifyFailed`  -> `-1` (false allow: shipped a break)
/// - Block      + `BlockedAndAccepted` -> `0` (a needed stop; neutral, no proof either way)
/// - Block      + `BlockedThenOverriddenClean` -> `-1` (false block: cost the dev a fight)
/// - Ask        + `BlockedAndAccepted` -> `0` (abstention that held)
#[must_use]
pub const fn label(action: Action, outcome: VerifyOutcome) -> f64 {
    // A false decision in EITHER direction is the costly error (-1): a false
    // allow shipped a break, and a false block (overridden, then verified clean)
    // cost the dev a fight. Scoring them equally is the reward-hacking guard.
    if is_false_decision(action, outcome) {
        return -1.0;
    }
    // A change that went through and verified clean is the win (+1).
    if matches!(action, Action::Allow | Action::Ask)
        && matches!(outcome, VerifyOutcome::VerifiedClean)
    {
        return 1.0;
    }
    // Everything else — a block/ask that stood, or an inconsistent log — is a
    // neutral abstention: no counterfactual, so no reward and no penalty.
    0.0
}

/// The wire-enum tag (`kavach_patterns::Reward`, `snake_case`) for a labeled
/// decision — the string `update_bandit_reward` writes back into `bandit_log`.
///
/// Derived from the SAME [`label`] scalar so the tag and the OPE scalar can never
/// disagree: `+1` ⇒ `verified_clean`, `0` ⇒ `needed_ask`, `-1` ⇒ `false_decision`.
#[must_use]
pub fn reward_tag(action: Action, outcome: VerifyOutcome) -> &'static str {
    let scalar = label(action, outcome);
    if scalar > 0.0 {
        "verified_clean"
    } else if scalar < 0.0 {
        "false_decision"
    } else {
        "needed_ask"
    }
}

/// The two costly false decisions, scored `-1`: a false allow (shipped a break)
/// and a false block (overridden, then verified clean).
const fn is_false_decision(action: Action, outcome: VerifyOutcome) -> bool {
    matches!(
        (action, outcome),
        (Action::Allow, VerifyOutcome::VerifyFailed)
            | (_, VerifyOutcome::BlockedThenOverriddenClean)
    )
}

#[cfg(test)]
#[path = "label_test.rs"]
mod tests;
