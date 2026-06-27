//! Pure, non-AI attack-lens heuristics — the shared kernel for the Meta-Harness
//! Loophole Loop.
//!
//! ONE source of truth for "what is a suspected loophole on this line", reused by
//! BOTH the CLI sweep (`kavach loophole sweep|loop`) and the engine Stop-gate hook
//! (bounded per-turn scan of changed files). Kavach only DETECTS + RECORDS via
//! these heuristics; it NEVER calls an LLM — the native subscription agent does
//! every fix.
//!
//! The six lenses mirror CLAUDE.md `loophole_self_interrogation`. Heuristics are
//! intentionally conservative (a hint, not a proof): the agent that picks up the
//! finding does the real root-cause. First match wins so one line yields at most
//! one finding (keeps the (lens,site) key unambiguous).
/// One adversarial attack lens.
///
/// Each names a failure mode the happy path never exercises; the scan runs every
/// lens over the target and records what breaks.
// INTENTIONALLY EXHAUSTIVE: the six lenses are a closed set (CLAUDE.md
// loophole_self_interrogation). Downstream `from_kernel` matches MUST break to
// compile if a lens is ever added — that compile error is the one-source-of-truth
// guarantee, so `#[non_exhaustive]` (which would force a silent `_ =>`) is wrong.
#[expect(
    clippy::exhaustive_enums,
    reason = "closed 6-lens set; downstream matches must fail on additions"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lens {
    /// Two actors at once → TOCTOU / lost-update / double-claim.
    Concurrency,
    /// Process dies mid-op → orphaned lock / half-write / leaked task.
    Failure,
    /// null / huge / wrong-type / hostile input → panic / injection.
    Malformed,
    /// Caller without rights → missing check / confused-deputy / IDOR.
    Authz,
    /// Same request twice → non-idempotent mutation.
    Replay,
    /// empty / max / negative / off-by-one.
    Boundary,
}
impl Lens {
    /// Every lens, in the canonical order the sweep runs them.
    pub const ALL: [Self; 6] = [
        Self::Concurrency,
        Self::Failure,
        Self::Malformed,
        Self::Authz,
        Self::Replay,
        Self::Boundary,
    ];
    /// Stable kebab slug (also the mistakes-row + card-key fragment).
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Concurrency => "concurrency",
            Self::Failure => "failure",
            Self::Malformed => "malformed",
            Self::Authz => "authz",
            Self::Replay => "replay",
            Self::Boundary => "boundary",
        }
    }
}
/// One suspected loophole: which lens, which 1-based line, and a human hint.
///
/// The file path is supplied by the caller (the kernel scans text, not paths).
// INTENTIONALLY EXHAUSTIVE: a plain data carrier callers both build and
// destructure by field; `#[non_exhaustive]` would block struct-literal use here.
#[expect(
    clippy::exhaustive_structs,
    reason = "plain finding record; callers construct and field-match it"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LensFinding {
    /// Which attack lens fired.
    pub lens: Lens,
    /// 1-based line number within the scanned text.
    pub line: usize,
    /// Conservative hint describing the suspected failure mode.
    pub hint: &'static str,
}
/// Run every lens over `source` (a Rust file's text) and collect hints.
///
/// Stops at the file's `#[cfg(test)]` boundary: test code legitimately uses
/// `unwrap`/`expect`/index, so scanning it floods the board with non-defects.
#[must_use]
pub fn scan_text(source: &str) -> Vec<LensFinding> {
    let mut out = Vec::new();
    for (idx, raw) in source.lines().enumerate() {
        let line = idx.saturating_add(1);
        let l = raw.trim();
        // The conventional test-module marker → stop; everything below is tests.
        if l.starts_with("#[cfg(test)]") {
            break;
        }
        // Comments are guidance, not code — never flag them (kills the obvious
        // false positive of matching a pattern named in a doc comment).
        if l.starts_with("//") || l.starts_with('*') {
            continue;
        }
        if let Some((lens, hint)) = classify(l) {
            out.push(LensFinding { lens, line, hint });
        }
    }
    out
}
/// Map a single code line to a lens + hint, or `None`. First match wins so one
/// line yields at most one finding.
#[must_use]
pub fn classify(l: &str) -> Option<(Lens, &'static str)> {
    // failure: a discarded Result/Option on a fallible op → silent error path.
    if l.contains("let _ =") && (l.contains('?') || l.contains("await") || l.contains("()")) {
        return Some((
            Lens::Failure,
            "discarded Result with `let _ =` — error path swallowed",
        ));
    }
    // malformed: unwrap/expect on external input → panic on hostile input.
    if (l.contains(".unwrap()") || l.contains(".expect(")) && !l.contains("#[") {
        return Some((
            Lens::Malformed,
            "unwrap/expect — panics on malformed/unexpected input",
        ));
    }
    // boundary: raw index/slice → panic on empty/oob.
    if l.contains("[0]") || l.contains(".first().unwrap") {
        return Some((
            Lens::Boundary,
            "direct index/[0] — panics on empty/out-of-bounds",
        ));
    }
    // replay: a SQL INSERT without an idempotency key. We match ONLY uppercase
    // `INSERT` (SQL text) — a Rust `.insert(` on a local HashMap/HashSet/Vec is
    // ordinary in-memory mutation, not the non-idempotent persisted write this
    // lens means, and flagging it floods the board (the gates/loader/router FP
    // cluster). SQL inserts live in query strings, which is what we keep.
    if l.contains("INSERT") && !l.contains("upsert") && !l.contains("ON CONFLICT") {
        return Some((
            Lens::Replay,
            "SQL INSERT without idempotency key — re-run may duplicate",
        ));
    }
    // concurrency: a DB-level check-then-act (existence probe before a write). We
    // match `exists`/`EXISTS` only — a bare Rust `.contains()` on a local
    // collection is single-threaded membership, not a cross-actor TOCTOU, so
    // matching it produces false positives with near-zero real yield.
    if l.contains("if !") && (l.contains("exists") || l.contains("EXISTS")) {
        return Some((
            Lens::Concurrency,
            "check-then-act — TOCTOU window between existence check and write",
        ));
    }
    // authz: a handler/route with no visible authorize/allow call on the line.
    if (l.contains("pub async fn") || l.contains("pub fn")) && l.contains("handler") {
        return Some((
            Lens::Authz,
            "handler signature — verify an authorize/allow check guards it",
        ));
    }
    None
}
#[cfg(test)]
#[path = "loophole_lens_test.rs"]
mod tests;
