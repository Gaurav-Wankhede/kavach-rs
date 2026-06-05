// db-ops-exempt: pure action+outcome→reward-tag mapping, no DB access at all.
//! Pure grading map for the P3a back-fill — split from the hub to stay under the
//! LOC ceiling and test the mapping without a store.

use kavach_ope::Action;
use kavach_ope::label::{VerifyOutcome, reward_tag};

#[cfg(test)]
#[path = "grade_test.rs"]
mod tests;

/// The reward tag for one logged row given the session verify outcome, or `None`
/// if the payload is malformed / carries no known action (a surfaced skip).
#[must_use]
pub(super) fn reward_tag_for_row(payload: &str, verified_clean: bool) -> Option<&'static str> {
    let action = action_of_payload(payload)?;
    Some(reward_tag(action, outcome_for(action, verified_clean)))
}

/// The per-action verify outcome implied by the session-level pass/fail. A
/// passing session ⇒ `VerifiedClean`. A FAILING session only proves a false
/// ALLOW; a `Block`/`Ask` has no counterfactual ⇒ neutral `BlockedAndAccepted`
/// (`label` scores it `0`), never an unprovable penalty.
const fn outcome_for(action: Action, verified_clean: bool) -> VerifyOutcome {
    if verified_clean {
        return VerifyOutcome::VerifiedClean;
    }
    match action {
        Action::Allow => VerifyOutcome::VerifyFailed,
        // Ask/Block (and any future variant) in a failing session: no
        // counterfactual ⇒ neutral. Action is #[non_exhaustive] ⇒ wildcard.
        Action::Ask | Action::Block | _ => VerifyOutcome::BlockedAndAccepted,
    }
}

/// Read the `action` field off a `BanditRow` JSON, mapped to the OPE action.
fn action_of_payload(payload: &str) -> Option<Action> {
    let v: serde_json::Value = serde_json::from_str(payload).ok()?;
    match v.get("action")?.as_str()? {
        "allow" => Some(Action::Allow),
        "ask" => Some(Action::Ask),
        "block" => Some(Action::Block),
        _ => None,
    }
}
