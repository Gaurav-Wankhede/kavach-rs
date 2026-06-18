//! Compaction-seam reconcile predicate (E7).
//!
//! ROOT CAUSE: auto-compact can fire in the window AFTER a code edit but BEFORE
//! its `kavach db status-update`. The post-compact session then sees the card
//! still `in_progress`, a dirty working tree, but NO status transition recorded —
//! the work is done-but-unrecorded. The prior handling was indirect (`SessionStart`
//! re-reads the board, the E4 stale-claim sweep reclaims a genuinely-abandoned
//! lease, the witness-gate refuses a false `done`), but there was no EXPLICIT
//! step that says "the dirty files match the in-progress card → resume at verify,
//! don't re-edit." It relied on the agent re-deriving that from context.
//!
//! THE FIX (pure predicate, unit-tested here): given the in-progress card's
//! expected paths, the porcelain `git status`, and whether a status command has
//! run since the claim, decide [`ReconcileAction::ResumeVerify`] (dirty files
//! intersect the card's paths and nothing was recorded yet → finish verifying the
//! already-done work) vs [`ReconcileAction::ReDispatch`] (no overlap, or a status
//! cmd already ran → treat as a normal fresh dispatch). The impure `git status`
//! read + board lookup live in the caller; this stays pure so the seam is provable
//! without spawning git. SOURCE: `decision.harness.state_lives_in_store` + E4.

#[cfg(test)]
#[path = "reconcile_test.rs"]
mod tests;

/// What `SessionStart` should do with an `in_progress` card after a possible
/// compaction seam. `#[non_exhaustive]` so a future third action (e.g. an
/// ambiguous "ask") can be added without breaking match arms downstream.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReconcileAction {
    /// Dirty tree overlaps the card's expected paths AND no status-update ran
    /// since the claim → the work is done-but-unrecorded; resume at the VERIFY
    /// step (run the witnesses + close), do NOT re-edit from scratch.
    ResumeVerify,
    /// No overlap, an empty tree, or a status command already ran → there is no
    /// orphaned edit to recover; proceed with normal dispatch.
    ReDispatch,
}

/// Extract the per-card expected-paths hint from card content. A card declares the
/// files its work touches on a `TOUCHES: <p1> <p2> …` line (whitespace- or
/// comma-separated), the same opt-in convention as `WITNESS_ROOT:`/`DEPENDS_ON:`.
/// Absent line → empty vec (the predicate then conservatively re-dispatches, never
/// a false resume). The first `TOUCHES:` line wins; tokens are trimmed, empties
/// dropped. Paths are matched by BASENAME downstream, so a bare filename suffices.
#[must_use]
pub(crate) fn touched_paths_from_card(content: &str) -> Vec<String> {
    content
        .lines()
        .find_map(|raw| raw.trim().strip_prefix("TOUCHES:"))
        .map(|rest| {
            rest.split([' ', '\t', ','])
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Basename of a path (`a/b/c.rs` → `c.rs`); a bare name returns itself.
fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Extract the destination path from one porcelain `XY <path>` line, handling the
/// ` -> ` rename form (the live file is the destination). Empty/short → `None`.
/// Mirrors `stop::foreign_tree_logic::line_path`; kept local so this module is a
/// self-contained, separately-testable seam.
fn porcelain_path(line: &str) -> Option<&str> {
    let rest = line.get(3..)?.trim();
    if rest.is_empty() {
        return None;
    }
    Some(rest.rsplit(" -> ").next().unwrap_or(rest).trim())
}

/// True iff any dirty (porcelain) file's basename matches one of the card's
/// expected-path basenames — i.e. the orphaned edit belongs to THIS card.
fn dirty_overlaps_card(porcelain: &str, card_paths: &[String]) -> bool {
    if card_paths.is_empty() {
        return false;
    }
    let wanted: std::collections::HashSet<&str> =
        card_paths.iter().map(|p| basename(p)).collect();
    porcelain
        .lines()
        .filter_map(porcelain_path)
        .any(|p| wanted.contains(basename(p)))
}

/// THE reconcile predicate (E7). Decide whether an `in_progress` card is the
/// victim of a compaction seam (resume its verify) or a normal dispatch.
///
/// `card_in_progress` — the claimed card is still `in_progress` (not done/todo).
/// `status_cmd_since_claim` — a `kavach db status-update` for this card has run
///   since the claim (so the transition WAS recorded; no seam).
/// `porcelain` — `git status --porcelain` output.
/// `card_paths` — the card's `TOUCHES:` expected paths.
///
/// `ResumeVerify` iff: still `in_progress` AND no status cmd recorded AND the dirty
/// tree overlaps the card's paths. Every other combination is `ReDispatch` —
/// fail-safe: an absent `TOUCHES:` hint, a clean tree, or an already-recorded
/// transition can never trigger a false resume (which would skip re-doing genuine
/// work). Closing the loophole the other way (a real seam mis-read as `ReDispatch`)
/// is bounded by the witness-gate, which still refuses a false `done`.
#[must_use]
pub(crate) fn reconcile_action(
    card_in_progress: bool,
    status_cmd_since_claim: bool,
    porcelain: &str,
    card_paths: &[String],
) -> ReconcileAction {
    if card_in_progress
        && !status_cmd_since_claim
        && dirty_overlaps_card(porcelain, card_paths)
    {
        ReconcileAction::ResumeVerify
    } else {
        ReconcileAction::ReDispatch
    }
}

/// Impure `SessionStart` wrapper: read the live in-progress card for `project` and
/// the working-tree porcelain, run [`reconcile_action`], and return a `[RECONCILE]`
/// block ONLY when the verdict is [`ReconcileAction::ResumeVerify`] (the seam case).
///
/// Fail-soft: any RPC/git miss returns `None` (the block is simply omitted — a
/// session start is never blocked on this). The block, when present, instructs the
/// post-compact agent NOT to re-edit but to resume at the witness/verify step for
/// the named card — closing the done-but-unrecorded gap the prior design left to
/// agent re-derivation. SOURCE: E7 card VERIFY clause.
#[must_use]
pub(super) fn reconcile_context(project: &str) -> Option<String> {
    if project.is_empty() {
        return None;
    }
    let (key, content) = in_progress_card(project)?;
    let card_paths = touched_paths_from_card(&content);
    if card_paths.is_empty() {
        return None; // No TOUCHES: hint → cannot prove overlap; never a false resume.
    }
    let porcelain = git_status_porcelain()?;
    match reconcile_action(true, false, &porcelain, &card_paths) {
        ReconcileAction::ResumeVerify => Some(format!(
            "[RECONCILE]\ncard: {key}\nstate: in_progress + dirty tree overlaps its TOUCHES paths, \
             no status-update recorded since claim.\n\
             cause: auto-compact likely fired between the edit and its status-update — the work is \
             done-but-UNRECORDED.\n\
             action: do NOT re-edit from scratch. Resume at the VERIFY step — run the 3-witness \
             check on the existing changes, then close the card.\n"
        )),
        ReconcileAction::ReDispatch => None,
    }
}

/// Read the single in-progress roadmap card `(key, content)` for `project`, or
/// `None` on any RPC miss / empty result. Best-effort; reconcile is advisory.
fn in_progress_card(project: &str) -> Option<(String, String)> {
    let params = serde_json::json!({ "project": project });
    let v = kavach_rpc::client::call::<_, serde_json::Value>(
        "roadmap.list_in_progress_cards",
        Some(params),
    )
    .ok()?;
    let first = v.as_array()?.iter().next()?;
    let key = first.get("key").and_then(serde_json::Value::as_str)?;
    let content = first
        .get("content")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    Some((key.to_owned(), content.to_owned()))
}

/// `git status --porcelain` of the current checkout, or `None` if git is absent /
/// errors. Pure-string output is handed to the testable predicate.
fn git_status_porcelain() -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}
