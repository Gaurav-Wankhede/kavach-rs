//! Witness-gated auto-verify: promote `done` cards to `verified` only after the
//! shared workspace build+test witnesses pass (3-witness law; the diff witness
//! is implicit — the card reached `done` because its work shipped).
//!
//! `§NANO_FILE` split: the witness machinery (workspace discovery + cargo runs)
//! lives in the `witness` child; this hub keeps the orchestration.

pub(crate) mod trajectory_eval;
pub(crate) mod witness;

use witness::{WitnessRun, run_workspace_witnesses, witness_root_from_card};

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
    /// Witnesses PASSED but EVERY `verify_card` RPC failed to flip its card — the
    /// daemon is down or the write errored. This is NOT "nothing was done": the
    /// work is proven, the DB write is the thing that failed. Surfaced loudly so the
    /// loop never silently re-dispatches a finished card on a transient outage.
    VerifyRpcDown,
    /// Promoted this many `done -> verified`. Dependents may now be dispatchable.
    Promoted(usize),
}

/// Every roadmap card currently at `done` (work shipped, awaiting verification),
/// as `(key, content)` so the caller can read a per-card `WITNESS_ROOT:` hint.
/// Empty on any error — auto-verify is best-effort.
fn list_done_cards(project_slug: &str) -> Vec<(String, String)> {
    if project_slug.is_empty() {
        return Vec::new();
    }
    let params = serde_json::json!({ "project": project_slug });
    kavach_rpc::client::call::<_, serde_json::Value>("roadmap.list_done_cards", Some(params))
        .ok()
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|c| {
            let key = c.get("key").and_then(serde_json::Value::as_str)?;
            let content = c
                .get("content")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            Some((key.to_owned(), content.to_owned()))
        })
        .collect()
}

/// Outcome of one `verify_card` RPC — kept distinct so the caller can tell a
/// genuine "card was not at done" (`NotFlipped`) apart from a daemon/transport
/// outage (`RpcError`). Collapsing both to `false` is what let a transient DB
/// outage masquerade as "work not done" and re-dispatch a finished card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FlipResult {
    /// The card flipped `done -> verified` this call.
    Flipped,
    /// RPC succeeded but the card did not flip (already verified, or not at done).
    NotFlipped,
    /// The RPC itself failed (daemon down / transport error) — the write never
    /// reached the DB. NOT evidence the work is incomplete.
    RpcError,
}

/// Promote one `done` card to `verified`, distinguishing a real miss from an RPC
/// outage. Best-effort write; the tri-state lets the caller fail LOUD on outage
/// instead of silently re-dispatching a finished card.
fn verify_card(project_slug: &str, key: &str) -> FlipResult {
    if project_slug.is_empty() || key.is_empty() {
        return FlipResult::NotFlipped;
    }
    let params = serde_json::json!({ "project": project_slug, "key": key });
    kavach_rpc::client::call::<_, serde_json::Value>("roadmap.verify_card", Some(params)).map_or(
        FlipResult::RpcError,
        |v| {
            if v.get("verified").and_then(serde_json::Value::as_bool) == Some(true) {
                FlipResult::Flipped
            } else {
                FlipResult::NotFlipped
            }
        },
    )
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
    let done = list_done_cards(project_slug);
    if done.is_empty() {
        return AutoVerify::NothingDone;
    }
    // Per-card WITNESS_ROOT hint: if any done card declares the repo its code
    // lives in (a cross-repo harness card dispatched while CWD is elsewhere), the
    // witnesses run THERE. First declared root wins; absent → env/CWD discovery.
    let card_root = done
        .iter()
        .find_map(|(_, content)| witness_root_from_card(content));
    match run_workspace_witnesses(card_root.as_deref()) {
        WitnessRun::Passed => {
            let mut promoted = 0_usize;
            let mut rpc_error = false;
            let mut advisories = Vec::new();
            for (key, _) in &done {
                match verify_card(project_slug, key) {
                    FlipResult::Flipped => promoted = promoted.saturating_add(1),
                    FlipResult::NotFlipped => {}
                    FlipResult::RpcError => rpc_error = true,
                }
                if let Some(adv) = trajectory_eval::eval_advisory(project_slug, key) {
                    advisories.push(adv);
                }
            }
            for adv in advisories {
                eprintln!("{adv}");
            }
            if promoted == 0 && rpc_error {
                AutoVerify::VerifyRpcDown
            } else {
                AutoVerify::Promoted(promoted)
            }
        }
        WitnessRun::Failed | WitnessRun::SpawnError => AutoVerify::WitnessFailed,
        WitnessRun::Unprovable => AutoVerify::Unprovable,
    }
}
