//! Per-line detector arms: error-handling P0s (unwrap/panic/exit/casts), macro
//! P0/P1s, and the `allow(...)` suppression bans. Walks `content` line by line
//! and pushes one violation per regex hit at its 1-based line.
//!
//! The unconditional arms live in a static `(idx, severity, pattern, fix)` table
//! looped per line; the three context-sensitive arms (`process::exit` allowed in
//! `main.rs`, `allow(dead_code)` suppressed under serde) are handled explicitly.
//! Data-driving the table is the canonical `clippy::too_many_lines` remedy —
//! extract the repetition, keep only the genuine logic.
//! SOURCE: <https://rust-lang.github.io/rust-clippy/master/index.html> (`too_many_lines`)
use crate::severity::{Severity, Violation};
use regex::Regex;

fn push(v: &mut Vec<Violation>, sev: Severity, pat: &str, fix: &str, line: usize) {
    v.push(Violation::new(sev, pat, fix, line.saturating_add(1)));
}

/// `(regex index, severity, pattern name, fix hint)` for every arm that fires
/// purely on a regex hit with no extra context. Order is irrelevant — each line
/// is tested against all rows.
const ROWS: &[(usize, Severity, &str, &str)] = {
    use Severity::{P0Block, P1Advisory};
    &[
        (
            0,
            P0Block,
            "unwrap()",
            "Replace with ? or match. Invoke /error for propagation patterns",
        ),
        (
            1,
            P0Block,
            "panic!",
            "Return Result<T, E> instead. Invoke /error",
        ),
        (
            3,
            P1Advisory,
            "narrowing cast",
            "Replace with TryFrom::try_from() or .try_into()",
        ),
        (4, P1Advisory, "dbg!", "Remove dbg! macro before committing"),
        (
            5,
            P1Advisory,
            "print macro",
            "Replace with writeln!(io::stdout().lock(), ...) or tracing",
        ),
        (
            6,
            P0Block,
            "todo!/unimplemented!",
            "Implement the logic now — no stubs allowed",
        ),
        (
            7,
            P1Advisory,
            "unsafe block",
            "Add // SAFETY: comment explaining the invariant. Invoke /rust for unsafe code review",
        ),
        (
            9,
            P0Block,
            "allow(unused)",
            "Use the value or remove it — suppression hides debt. Run `kavach db write --category roadmap` if deferred",
        ),
        (
            10,
            P0Block,
            "allow(clippy::)",
            "Fix the clippy warning — suppression hides debt. Run `kavach db write --category roadmap` if deferred",
        ),
        (
            13,
            P0Block,
            "unwrap_or",
            "Propagate with map_err + ? to preserve error context",
        ),
        (
            14,
            P0Block,
            "unwrap_or_else",
            "Propagate with map_err + ? instead of swallowing errors",
        ),
        (
            15,
            P0Block,
            "unwrap_or_default",
            "Handle with match or ok_or + ? — defaults hide failures",
        ),
        (
            16,
            P0Block,
            "result.ok()",
            "Log or propagate the error — .ok() silently discards Err",
        ),
        (
            17,
            P0Block,
            "direct indexing",
            "Replace v[i] with .get(i) and handle None",
        ),
        (
            18,
            P0Block,
            "..Default::default()",
            "Destructure every field explicitly — hidden defaults mask unset fields",
        ),
        (
            19,
            P0Block,
            "_ => catch-all",
            "Enumerate every variant — catch-all hides unhandled cases",
        ),
    ]
};

/// Scan each line for single-line anti-patterns. `base` is the file basename
/// (so `process::exit` is allowed in `main.rs`); `has_serde` suppresses the
/// dead-code arm when the struct derives `Deserialize`.
pub(super) fn scan(
    r: &[Regex],
    content: &str,
    base: &str,
    has_serde: bool,
    v: &mut Vec<Violation>,
) {
    use Severity::P0Block;
    for (i, line) in content.lines().enumerate() {
        let hit = |idx: usize| r.get(idx).is_some_and(|re| re.is_match(line));
        for &(idx, sev, pat, fix) in ROWS {
            if hit(idx) {
                push(v, sev, pat, fix, i);
            }
        }
        // Context-sensitive arms.
        if hit(2) && base != "main.rs" {
            push(
                v,
                P0Block,
                "process::exit",
                "Return Err from main instead. Invoke /error",
                i,
            );
        }
        if hit(8) && !has_serde {
            push(
                v,
                P0Block,
                "allow(dead_code)",
                "Delete dead code or wire it. Run `kavach db write --category roadmap` if deferred",
                i,
            );
        }
    }
}
