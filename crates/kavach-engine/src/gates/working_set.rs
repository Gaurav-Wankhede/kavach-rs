//! Structurally-lossless working-set reconstruction for the compaction seam.
//!
//! The `PreCompact` guard snapshots + warns; this is its counterpart on the other side
//! of the seam. `PostCompact` rebuilds the EXACT durable spine from the DB (active card,
//! its `TOUCHES` paths, recent decisions) as structured text injected BEFORE the lossy
//! `COMPACT_SUMMARY`. The summary becomes a supplement, not the source of truth: every
//! working-set fact is re-derived losslessly from the store, never from a
//! summary-of-a-summary. This is the DAG-trim direction — re-inject structured state,
//! drop only redundant prose. See `decision.engine.lossless-working-set-reconstruction`.
//!
//! SOURCE: `arxiv.org/pdf/2602.22402` (structurally-lossless trimming),
//! `redis.io/blog/context-rot`.
// kavach:intentional cohesive compaction-seam reconstructor (one concern, RPC-backed)
use std::fmt::Write as _;
/// How many recent decision rows to re-inject. Bounded so the reconstruction stays a
/// tight spine, never a full dump that re-triggers compaction (the amnesia loop).
const RECENT_DECISIONS: usize = 8;
/// Rebuild the lossless `[WORKING_SET]` block from live DB state, or `None` when the
/// project is empty or the DB yields nothing (fail-soft — `PostCompact` then relies on
/// the summary alone, exactly as before). The block is exact + re-derivable: it names
/// the active card, its TOUCHES paths, and the recent decision keys+titles so the
/// post-compact turn reconstructs its state from the STORE, not the summary.
#[must_use]
pub(in crate::gates) fn reconstruct(project: &str) -> Option<String> {
    if project.is_empty() {
        return None;
    }
    let active = active_card(project);
    let decisions = recent_decisions(project);
    if active.is_none() && decisions.is_empty() {
        return None;
    }
    let mut out = String::from(
        "\n[WORKING_SET — LOSSLESS, re-derived from the store; trust this over [COMPACT_SUMMARY]]\n",
    );
    if let Some((key, title, touches)) = active {
        out.push_str(&render_intent_line(&title));
        write!(
            out,
            "active_card: {key}\n  touches: {touches}\n  resume: re-read the card \
             (`kavach db get --project {project} --category roadmap --key {key} --full`), \
             then VERIFY on those paths — do NOT re-implement.\n"
        )
        .ok();
    }
    if !decisions.is_empty() {
        out.push_str(
            "recent_decisions (settled — do NOT re-litigate; recall with kavach db get):\n",
        );
        for (key, title) in decisions {
            writeln!(out, "  - {key}: {title}").ok();
        }
    }
    out.push_str(
        "spine: the DECISION_MAP / PRACTICE_DELTA / PATTERN_DAG and [AUTONOMY_CONTRACT] are \
         re-injected every turn — obey them; do not re-derive from the summary.\n",
    );
    Some(out)
}
/// The active card's title rendered as the restored intent, or empty when blank.
/// The card TITLE *is* the work intent; re-emitting it kills the post-compact
/// amnesia loop — the model resumes the SAME goal, not a summary-of-a-summary.
#[must_use]
fn render_intent_line(title: &str) -> String {
    if title.is_empty() {
        return String::new();
    }
    format!("[INTENT_RESTORED] {title} — this is the active intent; resume it, do NOT re-derive.\n")
}
/// The single in-progress card as `(key, title, touches-string)`, or `None` on miss.
fn active_card(project: &str) -> Option<(String, String, String)> {
    let params = serde_json::json!({ "project": project });
    let v = kavach_rpc::client::call::<_, serde_json::Value>(
        "roadmap.list_in_progress_cards",
        Some(params),
    )
    .ok()?;
    let first = v.as_array()?.iter().next()?;
    let key = first.get("key").and_then(serde_json::Value::as_str)?;
    let title = first
        .get("title")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let content = first
        .get("content")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let paths = super::session_start::reconcile::touched_paths_from_card(content);
    let touches = if paths.is_empty() {
        "(none declared)".to_owned()
    } else {
        paths.join(" ")
    };
    Some((key.to_owned(), title.to_owned(), touches))
}
/// The most recent `RECENT_DECISIONS` decision rows as `(key, title)`, newest first;
/// empty on any RPC miss (fail-soft).
fn recent_decisions(project: &str) -> Vec<(String, String)> {
    let params = serde_json::json!({ "project": project, "category": "decision" });
    let Ok(v) = kavach_rpc::client::call::<_, serde_json::Value>("db.query", Some(params)) else {
        return Vec::new();
    };
    // db.query returns { entries: [ { key, title, ... } ] }.
    let Some(arr) = v.get("entries").and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|row| {
            let key = row.get("key").and_then(serde_json::Value::as_str)?;
            let title = row
                .get("title")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            Some((key.to_owned(), title.to_owned()))
        })
        .take(RECENT_DECISIONS)
        .collect()
}
#[cfg(test)]
#[path = "working_set_test.rs"]
mod tests;
