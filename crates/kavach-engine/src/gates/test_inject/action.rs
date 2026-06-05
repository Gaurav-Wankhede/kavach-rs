//! Escalation level by nudge count, and the crate-scoped `cargo nextest` action.
use super::path::crate_name_from_path;

/// Escalation phrasing keyed to how many times we've already nudged.
pub(super) const fn nudge_level(nudge: i32) -> &'static str {
    if nudge >= 10 {
        "URGENT"
    } else if nudge >= 6 {
        "strongly recommend (stop adding features)"
    } else if nudge >= 3 {
        "recommend"
    } else {
        "suggest"
    }
}

/// Build the test command scoped to the crates the pending files belong to.
/// Running `--workspace` on a 20-crate project takes 19+ minutes; `-p <crate>`
/// compiles and tests only what changed. Falls back to `--workspace` when no
/// file resolves to a crate (e.g. workspace-root paths).
pub(super) fn scoped_action(files: &[String]) -> String {
    let mut crate_names: Vec<String> = files
        .iter()
        .filter_map(|p| crate_name_from_path(p))
        .collect();
    crate_names.sort();
    crate_names.dedup();
    if crate_names.is_empty() {
        return "cargo nextest run --workspace".to_owned();
    }
    let flags = crate_names
        .iter()
        .map(|c| format!("-p {c}"))
        .collect::<Vec<_>>()
        .join(" ");
    format!("cargo nextest run {flags}")
}
