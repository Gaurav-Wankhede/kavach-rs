//! Shallow-research guard.
//!
//! Blocks a confidence verdict about a subsystem ("clean", "wired correctly",
//! "no defect", "verified") when the same message carries NO leaf-depth
//! evidence — no `file.rs:NN` citation and no `[RCA]` block.
//! Absence-of-found-defect at name/config depth (cargo tree, `rg -l`,
//! Cargo.toml) is NOT proof-of-absence; a clean verdict must cite the actual
//! call-site body it inspected.
//!
//! SOURCE: bug-hunt-loop §LAW (read-only suspicion is noise; a reproducing
//! artifact is signal) + the symmetric corollary enforced here — a *clean*
//! verdict is noise unless it cites the leaf it reached.

/// A subsystem-level clean/wired/verified verdict found in prose.
const VERDICT_CUES: &[&str] = &[
    "wired correctly",
    "wired right",
    "correctly wired",
    "is clean",
    "are clean",
    "surface clean",
    "proven clean",
    "verified clean",
    "no defect",
    "no bug",
    "not a defect",
    "not a bug",
    "nothing is silently",
    "no protection",
    "redundant, so",
    "covered elsewhere",
    "no suppressed",
    // Verdict vocab mandated by ~/.claude/CLAUDE.md verdict_needs_leaf_evidence
    // ("clean / wired / correct / no defect / safe / done"). Phrased to catch the
    // assertion ("is correct", "is safe") not the noun ("correct usage").
    "is correct",
    "are correct",
    "correct by construction",
    "is safe",
    "are safe",
    "fail-safe",
];

/// Cheap check: does the prose cite a concrete `something.rs:NN` source
/// location?
///
/// That is the minimum artifact of leaf-depth reading. Uses `match_indices` +
/// byte lookup on the trailing offset (no string slicing) so it can never panic
/// on a UTF-8 boundary; the extension markers are pure ASCII, so the byte just
/// past a match is a valid index to probe for a digit.
fn cites_file_line(msg: &str) -> bool {
    let bytes = msg.as_bytes();
    for ext in [".rs:", ".ts:", ".tsx:", ".py:", ".go:", ".js:"] {
        for (idx, _) in msg.match_indices(ext) {
            let after = idx.saturating_add(ext.len());
            if bytes.get(after).is_some_and(u8::is_ascii_digit) {
                return true;
            }
        }
    }
    false
}

/// Detect a shallow verdict.
///
/// Returns `Some(reason)` when the message asserts a subsystem clean/wired
/// verdict but cites no `file:line` and carries no `[RCA]` block — the
/// shallow-research signature. Returns `None` when the verdict is backed by
/// leaf-depth evidence, or when no verdict is being asserted at all.
#[must_use]
pub fn detect_shallow_verdict(msg: &str) -> Option<String> {
    let lower = msg.to_lowercase();
    let asserts_verdict = VERDICT_CUES.iter().any(|c| lower.contains(c));
    if !asserts_verdict {
        return None;
    }
    // Backed by either a concrete source location or a full RCA block → deep.
    if cites_file_line(msg) || msg.contains("[RCA]") {
        return None;
    }
    Some(
        "SHALLOW VERDICT: a 'clean / wired / no-defect' conclusion was asserted \
         without leaf-depth evidence (no `file.rs:NN` citation, no [RCA] block). \
         Name-level reads (cargo tree, `rg -l`, Cargo.toml) prove a symbol is \
         REFERENCED, not that its call-site body is correct or reachable. Open \
         the actual entry→…→logic call path and cite the file:line you read, or \
         drop the verdict."
            .to_owned(),
    )
}

#[cfg(test)]
#[path = "shallow_verdict_guard_test.rs"]
mod tests;
