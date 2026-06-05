//! Destructive CLI Guard — Shell-Level Protection (2026). Hub: canonicalize →
//! regex-set match. Types in `kinds`, rows in `patterns`.
//!
//! ALGO: quote-strip + whitespace-collapse, then bounded regex set | TIME O(n) per cmd.
//! REJECTED: AST shell parser (too heavy for write-time gate); naive substring
//! (defeated by `'r''m' -rf` quote obfuscation). TRADEOFF: regex misses semantic
//! context → ambiguous patterns emit Confirm so the host prompts, not hard-blocks.
//! The `CodeExecFlag` category closes the FN where a tool allowlisted by NAME
//! (rg/find/git/go) carries a flag that is itself a code-exec primitive.
//! SOURCES (2026-06): <https://github.com/Dicklesworthstone/destructive_command_guard>,
//! <https://blog.trailofbits.com/2025/10/22/prompt-injection-to-rce-in-ai-agents/>
mod kinds;
mod patterns;

pub use kinds::{DestructiveCategory, DestructiveHit, DestructiveSeverity};
use patterns::{PATTERNS, PatternRow};

pub(crate) fn canonicalize(cmd: &str) -> String {
    let stripped: String = cmd
        .chars()
        .filter(|c| !matches!(c, '\'' | '"' | '\\'))
        .collect();
    let mut out = String::with_capacity(stripped.len());
    let mut prev_space = false;
    for c in stripped.chars() {
        if c.is_whitespace() {
            if !prev_space {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(c);
            prev_space = false;
        }
    }
    out.trim().to_owned()
}

fn hit(row: &PatternRow, canonical: &str) -> DestructiveHit {
    let (cat, sev, pat, fix, _) = row;
    DestructiveHit {
        severity: *sev,
        category: *cat,
        pattern: pat,
        fix,
        canonical: canonical.to_owned(),
    }
}

const fn rank(s: DestructiveSeverity) -> u8 {
    match s {
        DestructiveSeverity::P0Block => 3,
        DestructiveSeverity::P1Confirm => 2,
        DestructiveSeverity::P2Warn => 1,
    }
}

/// Highest-severity matching pattern, or `None` if the command is clean.
pub fn inspect(cmd: &str) -> Option<DestructiveHit> {
    let canonical = canonicalize(cmd);
    if canonical.is_empty() {
        return None;
    }
    let mut best: Option<DestructiveHit> = None;
    for row in PATTERNS.iter().filter(|r| r.4.is_match(&canonical)) {
        let candidate = hit(row, &canonical);
        best = match best {
            Some(b) if rank(b.severity) >= rank(candidate.severity) => Some(b),
            _ => Some(candidate),
        };
    }
    best
}

/// Every matching pattern (for callers that surface all hits at once).
pub fn inspect_all(cmd: &str) -> Vec<DestructiveHit> {
    let canonical = canonicalize(cmd);
    if canonical.is_empty() {
        return vec![];
    }
    PATTERNS
        .iter()
        .filter(|r| r.4.is_match(&canonical))
        .map(|row| hit(row, &canonical))
        .collect()
}

#[cfg(test)]
#[path = "destructive_cli_guard_test.rs"]
mod tests;
