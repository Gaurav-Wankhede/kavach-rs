//! Read-only GitHub PR + review-thread probe via the `gh` CLI.
//!
//! Advisory-only: surfaces the open PR for the current branch and its count of
//! UNRESOLVED review threads, then points at `/pr-review`. NEVER comments, merges,
//! or resolves — per the PR-handling decision (detect + advise; /pr-review does the work).
//! Fails OPEN: `gh` absent / unauthenticated / no PR → `None`.
//! SOURCE: <https://cli.github.com/manual/gh_pr_view> (`--json reviewDecision,...`).

use std::process::Command;

/// Open-PR summary for the current branch.
pub(super) struct PrState {
    /// PR number (`#123`).
    pub number: u64,
    /// GitHub review decision (`REVIEW_REQUIRED` / `CHANGES_REQUESTED` / `APPROVED` / "").
    pub decision: String,
}

/// Probe the current branch's open PR. `None` when `gh` is unavailable or no PR exists.
pub(super) fn probe() -> Option<PrState> {
    let out = Command::new("gh")
        .args(["pr", "view", "--json", "number,reviewDecision,state"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    // Only OPEN PRs are actionable; a merged/closed PR is not a sync concern.
    if json.get("state").and_then(|s| s.as_str()) != Some("OPEN") {
        return None;
    }
    let number = json.get("number").and_then(serde_json::Value::as_u64)?;
    let decision = json
        .get("reviewDecision")
        .and_then(|d| d.as_str())
        .unwrap_or_default()
        .to_owned();
    Some(PrState { number, decision })
}
