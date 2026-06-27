//! Anti-bloatware guard: blocks a COMMENT that documents a removal instead of
//! just removing the thing.
//!
//! A "tombstone" comment (`// X removed`, `-- field abolished`, `# no longer used`)
//! duplicates what the deletion + git history already record. It is the residue
//! the LLM leaves when asked to remove something — visible-helpfulness over the
//! exact requested outcome (decision.bloatware.no-tombstone-comments).
//!
//! SOURCE: <https://arxiv.org/pdf/2604.00478> (sycophancy: form-of-helpfulness over outcome)
//!
//! Exactness keeps the false-positive set empty: only a COMMENT line (`//`/`--`/`#`
//! prefix after trim) carrying a removal-marker fires. The same word inside a
//! string literal, an identifier, or live code never does (proven in the test).

use crate::severity::{Severity, Violation};

/// Removal-marker phrases that turn a comment into a tombstone. Matched
/// case-insensitively as whole substrings of the comment body.
const TOMBSTONE_MARKERS: [&str; 8] = [
    "removed",
    "abolished",
    "deprecated",
    "no longer",
    "dropped below",
    "(formerly",
    "used to be",
    "historical on-disk",
];

/// Only governed source is policed (same surface as `dedup_guard` plus `.sql`):
/// tombstones in docs/notes/markdown are out of scope.
fn is_governed_path(path: &str) -> bool {
    let governed_tree = path.contains("crates/core/")
        || path.contains("crates/api/")
        || path.contains("crates/services/")
        || path.contains("crates/kavach-");
    let source_ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("rs") || e.eq_ignore_ascii_case("sql"));
    governed_tree && source_ext
}

/// Strip a line to its comment body iff the line IS a comment (`//`/`--`/`#`
/// after leading whitespace). Returns `None` for non-comment (live-code) lines —
/// that is what excludes string literals and identifiers from ever firing.
fn comment_body(line: &str) -> Option<&str> {
    let t = line.trim_start();
    for prefix in ["//", "--", "#"] {
        if let Some(rest) = t.strip_prefix(prefix) {
            return Some(rest);
        }
    }
    None
}

/// Block (`P0Block`) every comment line whose body carries a removal-marker.
/// The deletion + git history are the record — a tombstone comment is bloat.
#[must_use]
pub fn detect(file_path: &str, content: &str) -> Vec<Violation> {
    let mut out = Vec::new();
    if !is_governed_path(file_path) {
        return out;
    }
    for (i, line) in content.lines().enumerate() {
        let Some(body) = comment_body(line) else {
            continue;
        };
        let lower = body.to_lowercase();
        if let Some(marker) = TOMBSTONE_MARKERS.iter().find(|m| lower.contains(**m)) {
            out.push(Violation::new(
                Severity::P0Block,
                "tombstone comment (§bloatware)",
                format!(
                    "This comment documents a removal (\"{marker}\") — DELETE it. The \
                     deletion + git history are the record; a removal-note in source \
                     is bloat (decision.bloatware.no-tombstone-comments)."
                ),
                i.saturating_add(1),
            ));
        }
    }
    out
}

#[cfg(test)]
#[path = "bloatware_guard_test.rs"]
#[cfg(test)]
#[path = "bloatware_guard_test.rs"]
mod tests;