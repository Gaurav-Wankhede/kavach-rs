//! Witness-gated auto-verify: promote `done` cards to `verified` only after the
//! shared workspace build+test witnesses pass (3-witness law; the diff witness
//! is implicit — the card reached `done` because its work shipped).

/// Three-state outcome of an auto-verify pass. The caller MUST branch on this so
/// a witness-failing `done` card (real AI repair work) is never confused with a
/// genuinely empty/owner-gated queue (a legitimate clean stop) — collapsing both
/// to `0` is what made the stop gate loop forever on owner-gated backlogs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AutoVerify {
    /// No `done` cards existed — nothing to verify. If no card is dispatchable
    /// either, the queue is empty or every remainder is dependency/owner-gated:
    /// a clean stop is correct.
    NothingDone,
    /// `done` cards exist but the workspace witnesses FAILED — there is an
    /// AI-fixable keystone. The loop must command repair, never stop.
    WitnessFailed,
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

/// Run the objective build+test witnesses ONCE over the whole workspace. Any
/// failure (tools absent, timeout-class spawn error) returns false → the cards
/// STAY `done`, never fabricated to verified. CWD is the agent's project root.
fn workspace_witnesses_pass() -> bool {
    let check = std::process::Command::new("cargo")
        .args(["check", "--workspace", "--quiet"])
        .status();
    if !matches!(check, Ok(s) if s.success()) {
        return false;
    }
    let test = std::process::Command::new("cargo")
        .args(["nextest", "run", "--workspace", "--no-fail-fast"])
        .status();
    matches!(test, Ok(s) if s.success())
}

/// Witness-gated auto-verify: find every `done` card, run the shared workspace
/// witnesses ONCE, and on success promote each `done -> verified` so the loop
/// self-closes finished work and flows to the next task instead of halting on
/// `[ALL_BLOCKED]`. Promotion also unblocks dependents on the same stop pass.
///
/// Returns a three-state [`AutoVerify`] so the caller can tell a witness-failing
/// `done` card (AI repair work) apart from an empty/owner-gated queue (clean
/// stop). Collapsing both to `0` previously trapped the loop on owner-gated
/// backlogs (prod deploy / mig-apply / live tests the AI cannot run).
pub(crate) fn auto_verify_done_cards(project_slug: &str) -> AutoVerify {
    let done = list_done_card_keys(project_slug);
    if done.is_empty() {
        return AutoVerify::NothingDone;
    }
    // One shared witness pass gates ALL done cards. On failure none are promoted
    // and a real keystone exists — the caller must command repair, not stop.
    if !workspace_witnesses_pass() {
        return AutoVerify::WitnessFailed;
    }
    AutoVerify::Promoted(
        done.iter()
            .filter(|key| verify_card(project_slug, key))
            .count(),
    )
}
