//! `§DEDUP` recall-not-redefine guard: blocks re-DEFINING an object the file
//! already imports from a central crate.
//!
//! This is the "copy instead of recall" anti-pattern the reuse law forbids — if you
//! `use core_utils::AppConfig` and then write `struct AppConfig`, you have shadowed
//! the central type with a divergent local copy. Recall the import; never redefine.
//!
//! SOURCE: <https://martinfowler.com/bliki/SingleSourceOfTruth.html>
//! SOURCE: <https://doc.rust-lang.org/reference/items/use-declarations.html>
//!
//! Scope: governed `crates/{core,api,services}` only (mirrors `§CENTRALIZED_CONFIG`);
//! tests and non-Rust files are exempted by `detect()`. The signal is intra-file
//! and exact — an imported name re-bound by a local item definition — so the
//! false-positive set is the empty set (proven in `dedup_guard_test.rs`): a name
//! you imported and then defined is, by construction, a redefinition.
mod parse;

use crate::severity::{Severity, Violation};

/// Only `crates/{core,api,services}` are governed (same surface as the
/// `§CENTRALIZED_CONFIG` LAW); the harness, frontend, and tools are out of scope.
fn is_governed_path(path: &str) -> bool {
    path.contains("/crates/core/")
        || path.contains("/crates/api/")
        || path.contains("/crates/services/")
}

/// Block (`P0Block`) when a name imported via `use` is re-defined by a local item
/// in the same governed file: recall the import, don't redefine the object.
// into a small `Vec`, pass 2 flags any local definition whose name is in that set.
// O(n·k) worst case (k = distinct imports, tiny in practice); O(k) space. A hash
// set would shave the membership test but k is small enough that linear `contains`
// wins on constant factors and avoids a dependency. Mirrors the rust_guard leaves.
#[must_use]
pub fn detect(file_path: &str, content: &str) -> Vec<Violation> {
    let mut out = Vec::new();
    if !is_governed_path(file_path) {
        return out;
    }
    let imports: Vec<&str> = content.lines().filter_map(parse::imported_name).collect();
    if imports.is_empty() {
        return out;
    }
    for (i, line) in content.lines().enumerate() {
        if let Some(name) = parse::defined_name(line)
            && imports.contains(&name)
        {
            out.push(Violation::new(
                Severity::P0Block,
                "redefines imported object (§DEDUP)",
                format!(
                    "`{name}` is already imported in this file — recall the central \
                     definition, do not redefine it. Delete the local `{name}` and use \
                     the imported one (single source of truth)."
                ),
                i.saturating_add(1),
            ));
        }
    }
    out
}

#[cfg(test)]
#[path = "dedup_guard_test.rs"]
#[cfg(test)]
#[path = "dedup_guard_test.rs"]
mod tests;