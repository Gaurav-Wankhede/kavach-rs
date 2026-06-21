//! Post-write LOCAL auto-commit (W4) — realtime kanban write + `git commit`.
//!
//! Per `decision.git_sync.local-commit-supersede` (2026-06-22): after a verified
//! in-project write, advance the work-ledger in ONE place — the active kanban card
//! is touched (`in_progress` heartbeat) and the working tree is committed LOCALLY.
//! KEEPS the `git_sync` no-push/no-GitHub clause: this NEVER pushes — push stays
//! outward-facing and agent-driven via `/pr-review`.
//!
//! Both actions fail OPEN: any git or RPC error is swallowed so a VCS hiccup can
//! never block the post-write pipeline (mirrors `git_sync`'s read-only probes).
//! Escape hatch: `KAVACH_AUTOCOMMIT_OFF=1` disables the commit (CI / bisect).

use std::process::Command;

/// Env switch to disable the local auto-commit (set to any non-empty value).
const DISABLE_ENV: &str = "KAVACH_AUTOCOMMIT_OFF";

/// Realtime-touch the active kanban card and commit the working tree LOCALLY.
/// Returns a one-line `[AUTOCOMMIT]` receipt for the post-write block, or `None`
/// when disabled, outside a repo, or nothing committable. NEVER pushes.
pub(super) fn run(card_key: &str) -> Option<String> {
    if std::env::var_os(DISABLE_ENV).is_some_and(|v| !v.is_empty()) {
        return None;
    }
    // Realtime DB heartbeat: re-assert the active card is in_progress so the kanban
    // reflects live work even mid-turn. Fail-open: an RPC miss is silent.
    if !card_key.is_empty() {
        touch_card_in_progress(card_key);
    }
    commit_local()
}

/// Best-effort `roadmap.claim_card` re-assert (idempotent `in_progress`). Silent on
/// any transport error — the heartbeat is advisory, never load-bearing.
fn touch_card_in_progress(card_key: &str) {
    let session_id = kavach_session::resolved_session_id();
    let params = serde_json::json!({
        "project": current_project(),
        "key": card_key,
        "session_id": session_id,
    });
    drop(kavach_rpc::client::call::<_, serde_json::Value>(
        "roadmap.claim_card",
        Some(params),
    ));
}

/// Resolve the active project slug from the session (empty when unknown — the RPC
/// then no-ops, fail-open).
fn current_project() -> String {
    kavach_session::load_session_state()
        .ok()
        .flatten()
        .map(|s| s.project)
        .unwrap_or_default()
}

/// `git add -A && git commit` LOCALLY. No push. `None` outside a repo, on a clean
/// tree (nothing to commit), or any git error. Returns a `[AUTOCOMMIT]` receipt.
fn commit_local() -> Option<String> {
    // Stage everything; a failure here (not a repo, etc.) aborts fail-open.
    let staged = Command::new("git").args(["add", "-A"]).output().ok()?;
    if !staged.status.success() {
        return None;
    }
    // Nothing staged → clean tree → nothing to commit (avoid an empty-commit error).
    let diff = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .status()
        .ok()?;
    if diff.success() {
        return None; // exit 0 from --quiet means NO staged changes
    }
    let msg = "chore(kavach): auto-commit after verified write (local, no push)";
    let out = Command::new("git")
        .args(["commit", "-m", msg])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(format!(
        "[AUTOCOMMIT] committed locally (no push) — {msg}. Push is agent-driven via /pr-review."
    ))
}

#[cfg(test)]
#[path = "autocommit_tests.rs"]
mod tests;
