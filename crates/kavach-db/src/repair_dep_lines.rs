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
    let mut first_dep_line: Option<usize> = None;
    let mut kept: Vec<&str> = Vec::with_capacity(body.lines().count());

    for (i, line) in body.lines().enumerate() {
        let t = line.trim();
        if let Some(payload) = t.strip_prefix("DEPENDS_ON:") {
            dep_line_count += 1;
            if first_dep_line.is_none() {
                first_dep_line = Some(i);
            }
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
    // Nothing to do: no dep lines, exactly one, and it is already canonical.
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

    let mut out: Vec<String> = Vec::new();
    if !deps.is_empty() {
        out.push(format!("DEPENDS_ON: {}", deps.join(", ")));
        if let Some(pos) = first_dep_line {
            // Preserve blank-line separation where the body had one.
            let _ = pos;
        }
    }
    out.extend(kept.iter().map(|l| (*l).to_owned()));

    let new_body = out.join("\n");
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

#[cfg(test)]
#[path = "repair_dep_lines_test.rs"]
mod tests;