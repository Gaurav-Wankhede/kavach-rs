//! Witness-gated auto-verify: promote `done` cards to `verified` only after the
//! shared workspace build+test witnesses pass (3-witness law; the diff witness
//! is implicit — the card reached `done` because its work shipped).
//!
//! `§MICRO_FILE` split: the witness machinery (workspace discovery + cargo runs)
//! lives in the `witness` child; this hub keeps the orchestration.

pub(crate) mod witness;

use witness::{WitnessRun, run_workspace_witnesses};

/// Three-state outcome of an auto-verify pass. The caller MUST branch on this so
/// a witness-failing `done` card (real AI repair work) is never confused with a
/// genuinely empty queue (a legitimate clean stop) — collapsing both to `0` is
/// what made the stop gate loop forever.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AutoVerify {
    /// No `done` cards existed — nothing to verify. If no card is dispatchable
    /// either, the queue is empty or every remainder is dependency-blocked:
    /// a clean stop is correct.
    NothingDone,
    /// `done` cards exist but the workspace witnesses FAILED — there is an
    /// AI-fixable keystone. The loop must command repair, never stop.
    WitnessFailed,
    /// Work cannot be proven: not a Rust project, no `KAVACH_VERIFY_CMD`, so no
    /// witness can run. Do NOT promote; a genuine blocker requiring user decision.
    Unprovable,
    /// Promoted this many `done -> verified`. Dependents may now be dispatchable.
    Promoted(usize),
}

/// Keys of every roadmap card currently at `done` (work shipped, awaiting
/// verification). Empty on any error — auto-verify is best-effort.
fn list_done_card_keys(project_slug: &str) -> Vec<String> {
    if project_slug.is_empty() {
        return Vec::new();
    }
    let params = serde_json::json!({ "project": project_slug });
    kavach_rpc::client::call::<_, serde_json::Value>("roadmap.list_done_cards", Some(params))
        .ok()
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|c| c.get("key").and_then(serde_json::Value::as_str))
        .map(str::to_owned)
        .collect()
}

/// Promote one `done` card to `verified`. True iff it flipped this call.
/// Best-effort: a miss leaves the card at `done` (re-attempted next stop).
fn verify_card(project_slug: &str, key: &str) -> bool {
    if project_slug.is_empty() || key.is_empty() {
        return false;
    }
    let params = serde_json::json!({ "project": project_slug, "key": key });
    kavach_rpc::client::call::<_, serde_json::Value>("roadmap.verify_card", Some(params))
        .ok()
        .and_then(|v| v.get("verified").and_then(serde_json::Value::as_bool))
        .unwrap_or(false)
}

/// Witness-gated auto-verify: find every `done` card, run the shared workspace
/// witnesses ONCE, and on success promote each `done -> verified` so the loop
/// self-closes finished work and flows to the next task instead of halting on
/// `[ALL_BLOCKED]`. Promotion also unblocks dependents on the same stop pass.
///
/// Returns a four-state [`AutoVerify`] so the caller can tell apart witness-
/// failing `done` cards (AI repair) from an empty queue (clean stop) from
/// unprovable work (non-Rust + no `KAVACH_VERIFY_CMD`).
pub(crate) fn auto_verify_done_cards(project_slug: &str) -> AutoVerify {
    let done = list_done_card_keys(project_slug);
    if done.is_empty() {
        return AutoVerify::NothingDone;
    }
    match run_workspace_witnesses() {
        WitnessRun::Passed => AutoVerify::Promoted(
            done.iter()
                .filter(|key| verify_card(project_slug, key))
                .count(),
        ),
        WitnessRun::Failed | WitnessRun::SpawnError => AutoVerify::WitnessFailed,
        WitnessRun::Unprovable => AutoVerify::Unprovable,
    }
}
