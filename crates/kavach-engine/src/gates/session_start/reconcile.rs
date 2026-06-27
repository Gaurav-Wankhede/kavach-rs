//! Compaction-seam reconcile predicate (E7): decide resume-verify vs re-dispatch
//! for an `in_progress` card after a possible auto-compact seam.
#[cfg(test)]
#[path = "reconcile_test.rs"]
#[path = "reconcile_test.rs"]
mod tests;
/// What `SessionStart` does with an `in_progress` card after a possible seam.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReconcileAction {
    /// Dirty tree overlaps the card's paths, nothing recorded → resume at verify.
    ResumeVerify,
    /// No overlap / clean tree / already recorded → normal dispatch.
    ReDispatch,
}
/// Parse a card's `TOUCHES: <p1> <p2> …` line (ws/comma-sep); first line wins,
/// empties dropped. Matched by basename downstream, so a bare filename suffices.
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
fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}
/// Destination path of a porcelain `XY <path>` line (rename → dest); short → None.
fn porcelain_path(line: &str) -> Option<&str> {
    let rest = line.get(3..)?.trim();
    if rest.is_empty() {
        return None;
    }
    Some(rest.rsplit(" -> ").next().unwrap_or(rest).trim())
}
fn dirty_overlaps_card(porcelain: &str, card_paths: &[String]) -> bool {
    if card_paths.is_empty() {
        return false;
    }
    let wanted: std::collections::HashSet<&str> = card_paths.iter().map(|p| basename(p)).collect();
    porcelain
        .lines()
        .filter_map(porcelain_path)
        .any(|p| wanted.contains(basename(p)))
}
/// `ResumeVerify` iff `in_progress` AND no status cmd recorded AND dirty tree
/// overlaps the card's `TOUCHES` paths; every other combination is `ReDispatch`
/// (fail-safe — a missing hint / clean tree / recorded transition never resumes).
#[must_use]
pub(crate) fn reconcile_action(
    card_in_progress: bool,
    status_cmd_since_claim: bool,
    porcelain: &str,
    card_paths: &[String],
) -> ReconcileAction {
    if card_in_progress && !status_cmd_since_claim && dirty_overlaps_card(porcelain, card_paths) {
        ReconcileAction::ResumeVerify
    } else {
        ReconcileAction::ReDispatch
    }
}
/// Impure wrapper: emit a `[RECONCILE]` block only in the seam case. Fail-soft —
/// any RPC/git miss or non-seam verdict returns `None`. Called by BOTH the
/// session-start hook AND the Stop gate: an auto-compact can fire a Stop before the
/// next session-start reconciles, so the Stop terminal also checks the seam (single
/// shared predicate, no second copy). See decision.harness.autocompact-stop-seam-unified.
#[must_use]
pub(in crate::gates) fn reconcile_context(project: &str) -> Option<String> {
    if project.is_empty() {
        return None;
    }
    let (key, content) = in_progress_card(project)?;
    let card_paths = touched_paths_from_card(&content);
    if card_paths.is_empty() {
        return None;
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
/// The single `in_progress` roadmap card `(key, content)`, or `None` on RPC miss.
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
fn git_status_porcelain() -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}
