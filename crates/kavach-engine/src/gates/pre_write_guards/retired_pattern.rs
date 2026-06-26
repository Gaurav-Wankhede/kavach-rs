//! `[RETIRED_PATTERN]` enforcement teeth — the precision layer that makes the
//! `[PATTERN_DAG]`/`[DECISION_MAP]` diagrams BINDING, not just informative.
//!
//! When a Write/Edit reintroduces a pattern the ledger already RETIRED (the
//! target of a `supersedes` edge), this gate BLOCKS and cites the replacement,
//! so the model cannot regress to a worst-practice the codebase moved past. The
//! retired set is fetched live over RPC; a daemon/DB blip ⇒ no retired set ⇒ no
//! block (fail-soft — never weaker than silence, never a false hard-stop).
//! SOURCE: roadmap.unit.mermaid-decision-architecture (Task F).
use crate::gates::pre_write_context::WriteContext;
use kavach_rpc::methods::db::RetiredPattern;

/// A retired pattern must carry a long-enough distinctive token before we match
/// on it, or a short generic title would false-positive on unrelated code.
const MIN_DISTINCTIVE_LEN: usize = 12;

/// Block the write iff its post-edit body reintroduces a retired pattern's
/// distinctive marker. `None` (allow) on test files, non-code, empty retired
/// set, or any RPC failure. Returns the block reason citing the replacement.
#[must_use]
pub(super) fn check(ctx: &WriteContext<'_>, project: &str) -> Option<String> {
    if ctx.is_test || !ctx.is_code || project.is_empty() {
        return None;
    }
    let retired = fetch_retired(project);
    if retired.is_empty() {
        return None;
    }
    let body = ctx.effective_content.to_lowercase();
    for rp in &retired {
        if distinctive_marker(&rp.retired).is_some_and(|m| body.contains(&m)) {
            return Some(format!(
                "[RETIRED_PATTERN] this change reintroduces \"{}\" — a pattern THIS \
                     codebase already RETIRED (see the [PATTERN_DAG] `-.retires.->` edge). \
                     Adopt the replacement instead: \"{}\". If the retirement is genuinely \
                     wrong, FILE a superseding decision/pattern row first, then proceed.",
                rp.retired, rp.replacement
            ));
        }
    }
    None
}

/// Fetch the project's retired-pattern set over RPC; empty on any failure.
fn fetch_retired(project: &str) -> Vec<RetiredPattern> {
    let params = serde_json::json!({ "project_slug": project });
    kavach_rpc::client::call("db.retired_patterns", Some(params)).unwrap_or_default()
}

/// The lowercased distinctive marker of a retired pattern's title: the "name"
/// half before the first rationale separator (`:` or em-dash `—`), trimmed.
/// Intra-name hyphens are KEPT (`dioxus-0.7` stays whole). `None` when too short
/// to match safely.
fn distinctive_marker(title: &str) -> Option<String> {
    let head = title
        .split([':', '—'])
        .next()
        .unwrap_or(title)
        .trim()
        .to_lowercase();
    (head.len() >= MIN_DISTINCTIVE_LEN).then_some(head)
}

#[cfg(test)]
#[path = "retired_pattern_test.rs"]
mod retired_pattern_test;
