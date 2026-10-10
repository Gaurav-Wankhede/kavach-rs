//! Repair the duplicate `DEPENDS_ON:` lines the two mirrors minted.
//!
//! ## The defect
//!
//! `mirror_depends_on_into_content` existed on BOTH the CLI and the daemon. Each
//! checked "does the content already declare this dep?" by LITERAL token match,
//! but dep targets reach the mirror in two forms — the CLI qualifies flag targets
//! (`kavach-rs/roadmap/foo`) while the NLU extractor emits bare keys (`foo`). A
//! line already carrying the qualified form never matched the incoming bare one,
//! so the other mirror appended a SECOND `DEPENDS_ON:` line. 52 roadmap rows
//! carry exactly that pair.
//!
//! ## The trap
//!
//! A row can have MORE targets on the qualified line than on the bare one. Real
//! example (`model-route-autoswitch-and-category`):
//!
//! ```text
//! DEPENDS_ON: kavach-rs/roadmap/model, kavach-rs/roadmap/model-shift-router-advisory
//! DEPENDS_ON: model-shift-router-advisory
//! ```
//!
//! "Drop the qualified line" would silently DESTROY the `model` dependency. So
//! the repair MERGES: collect every token from every `DEPENDS_ON:` line, normalize
//! each to its bare tail, dedupe, and emit ONE line. Nothing is dropped except
//! exact duplicates of the same dependency in two spellings.
//!
//! ## Idempotence
//!
//! Re-running on an already-clean body is a no-op: the merged set equals the
//! existing set, so the emitted line is byte-identical.

/// Bare tail of a dep key: strip up to and including the last `/`.
fn bare_tail(key: &str) -> &str {
    key.rsplit('/').next().unwrap_or(key).trim()
}

/// Split a `DEPENDS_ON:` payload into bare-tail keys, preserving first-seen order.
fn parse_dep_line(payload: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for tok in payload.split([',', ' ', '\t']) {
        let t = tok.trim();
        if t.is_empty() || t == "DEPENDS_ON" {
            continue;
        }
        let bare = bare_tail(t).to_owned();
        if !out.contains(&bare) {
            out.push(bare);
        }
    }
    out
}

/// One row's repair outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairReport {
    pub entry_key: String,
    /// `true` when the body actually changed.
    pub changed: bool,
    /// Dependencies before (deduped bare keys).
    pub deps_before: usize,
    /// Dependencies after.
    pub deps_after: usize,
}

/// Rewrite `body` so every `DEPENDS_ON:` declaration is one canonical line of
/// bare keys. Returns `(new_body, RepairReport)`; the body is returned unchanged
/// when there is nothing to repair.
pub fn repair_dep_lines(entry_key: &str, body: &str) -> (String, RepairReport) {
    let mut deps: Vec<String> = Vec::new();
    let mut dep_line_count = 0usize;
    let mut kept: Vec<&str> = Vec::with_capacity(body.lines().count());

    for line in body.lines() {
        let t = line.trim();
        if let Some(payload) = t.strip_prefix("DEPENDS_ON:") {
            dep_line_count += 1;
            for d in parse_dep_line(payload) {
                if !deps.contains(&d) {
                    deps.push(d);
                }
            }
            // Every dep line is dropped; one canonical line is re-emitted.
            continue;
        }
        kept.push(line);
    }

    let deps_before = deps.len();
    // Nothing declared at all — leave the body completely alone. Without this
    // guard the join below would rewrite EVERY dep-free row merely to drop its
    // trailing newline, so a dry run would list hundreds of clean rows as
    // "changed" and `--apply` would churn them all.
    if dep_line_count == 0 {
        return (
            body.to_owned(),
            RepairReport {
                entry_key: entry_key.to_owned(),
                changed: false,
                deps_before: 0,
                deps_after: 0,
            },
        );
    }
    // Already canonical: exactly one dep line whose parsed set equals the
    // collected set. Byte-for-byte unchanged.
    let already_canonical = dep_line_count == 1
        && deps_before > 0
        && body
            .lines()
            .find(|l| l.trim().starts_with("DEPENDS_ON:"))
            .is_some_and(|l| {
                let payload = l.trim().strip_prefix("DEPENDS_ON:").unwrap_or_default();
                parse_dep_line(payload) == deps
            });
    if already_canonical {
        return (
            body.to_owned(),
            RepairReport {
                entry_key: entry_key.to_owned(),
                changed: false,
                deps_before,
                deps_after: deps_before,
            },
        );
    }

    // Preserve the body's trailing-newline convention so a repair never alters
    // a byte the author did not ask us to touch.
    let trailing_newline = body.ends_with('\n');
    let mut out: Vec<String> = Vec::new();
    if !deps.is_empty() {
        out.push(format!("DEPENDS_ON: {}", deps.join(", ")));
    }
    out.extend(kept.iter().map(|l| (*l).to_owned()));

    let mut new_body = out.join("\n");
    if trailing_newline && !new_body.ends_with('\n') {
        new_body.push('\n');
    }
    // Guard: never return a body that lost a dependency.
    debug_assert!(parse_all(&new_body).len() == deps_before);
    let changed = new_body != body;
    (
        new_body,
        RepairReport {
            entry_key: entry_key.to_owned(),
            changed,
            deps_before,
            deps_after: deps.len(),
        },
    )
}

/// Every bare dep key declared anywhere in `body`.
fn parse_all(body: &str) -> Vec<String> {
    let mut deps: Vec<String> = Vec::new();
    for line in body.lines() {
        if let Some(payload) = line.trim().strip_prefix("DEPENDS_ON:") {
            for d in parse_dep_line(payload) {
                if !deps.contains(&d) {
                    deps.push(d);
                }
            }
        }
    }
    deps
}

/// A `DEPENDS_ON` token that is prose, not a card key.
///
/// ## Why this is a SHAPE rule, not a word list
///
/// These tokens were harvested by the NLU prose scanner ("depends on their
/// model", "after dedup", "blocks rename") and were permanently mirrored into
/// card bodies as if they were dependencies. A stop-list cannot keep pace —
/// English is unbounded. So the test is structural: a real kavach card key
/// ALWAYS carries a slug separator (kebab/dot/underscore) or is project-qualified.
/// Flat lowercase alphabetic text is prose.
///
/// ## What this deliberately does NOT remove
///
/// A qualified key (`kavach-rs/roadmap/x`) and any token with a separator are
/// kept even when the target is absent — an absent-but-real key is a genuine
/// typo to fix, not garbage to delete. Deleting those would silently drop live
/// edges. Only tokens that cannot be a key under any spelling are removed.
#[must_use]
pub fn is_prose_token(tok: &str) -> bool {
    let t = tok.trim();
    // Empty or obviously not a key.
    if t.is_empty() {
        return false;
    }
    // A separator anywhere means it was written as a key — absent or not, it is
    // a legitimate (if unresolvable) reference.
    if t.contains('-') || t.contains('.') || t.contains('_') || t.contains('/') {
        return false;
    }
    // Flat lowercase alphabetic => English prose. This is the NLU-harvest shape.
    t.chars().all(|c| c.is_ascii_alphabetic()) && t.chars().all(|c| !c.is_uppercase())
}

/// Drop prose tokens from every `DEPENDS_ON:` line, keeping genuine keys.
///
/// Returns `(new_body, removed)` where `removed` lists each dropped token so the
/// operator can see exactly what was discarded — a silent deletion of a
/// "dependency" is the same silent-wrongness class as the phantom node itself.
pub fn strip_prose_deps(body: &str) -> (String, Vec<String>) {
    let mut removed: Vec<String> = Vec::new();
    let mut deps: Vec<String> = Vec::new();
    for line in body.lines() {
        if let Some(payload) = line.trim().strip_prefix("DEPENDS_ON:") {
            for tok in payload.split([',', ' ', '\t']) {
                let t = tok.trim();
                if t.is_empty() || t == "DEPENDS_ON" {
                    continue;
                }
                let bare = bare_tail(t).to_owned();
                if is_prose_token(&bare) {
                    if !removed.contains(&bare) {
                        removed.push(bare);
                    }
                    continue;
                }
                if !deps.contains(&bare) {
                    deps.push(bare);
                }
            }
        }
    }
    if removed.is_empty() {
        return (body.to_owned(), removed);
    }

    // Rebuild: drop every dep line, re-emit at most one carrying the survivors.
    // If ALL deps were prose the card simply has no dependencies.
    let kept: Vec<&str> = body
        .lines()
        .filter(|l| !l.trim().starts_with("DEPENDS_ON:"))
        .collect();
    let mut out: Vec<String> = Vec::new();
    if !deps.is_empty() {
        out.push(format!("DEPENDS_ON: {}", deps.join(", ")));
    }
    out.extend(kept.iter().map(|l| (*l).to_owned()));
    let trailing = body.ends_with('\n');
    let mut new_body = out.join("\n");
    if trailing && !new_body.ends_with('\n') {
        new_body.push('\n');
    }
    (new_body, removed)
}

#[cfg(test)]
#[path = "repair_dep_lines_test.rs"]
mod tests;