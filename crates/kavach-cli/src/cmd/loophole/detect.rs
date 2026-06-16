//! Pure, non-AI lens heuristics for the Meta-Harness Loophole Loop. Each detector
//! scans one source file's text for a tell-tale of its lens's failure mode and
//! returns the matching line numbers. Heuristics are intentionally conservative
//! (a hint, not a proof) — the agent that picks up the heal card does the real
//! root-cause. Kavach only DETECTS + RECORDS; it never calls an LLM.
//! SOURCE: decision.meta.loophole-loop-extends-goal-yaml · CLAUDE.md `loophole_self_interrogation`.

use crate::cmd::goal::Lens;

/// One suspected loophole: which lens, which file, which 1-based line, and a hint.
pub(super) struct Finding {
    pub lens: Lens,
    pub file: String,
    pub line: usize,
    pub hint: String,
}

/// Run every lens over `source` (a Rust file's text at `path`) and collect hints.
/// Stops at the file's `#[cfg(test)]` boundary: test code legitimately uses
/// `unwrap`/`expect`/index, so scanning it floods the board with non-defects
/// (the noise loophole that surfaced on the first real sweep).
pub(super) fn scan_file(path: &str, source: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    for (idx, raw) in source.lines().enumerate() {
        let line_no = idx.saturating_add(1);
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
            out.push(Finding {
                lens,
                file: path.to_owned(),
                line: line_no,
                hint,
            });
        }
    }
    out
}

/// Map a single code line to a lens + hint, or `None`. First match wins so one
/// line yields at most one finding (keeps the (lens,site) key unambiguous).
fn classify(l: &str) -> Option<(Lens, String)> {
    // failure: a discarded Result/Option on a fallible op → silent error path.
    if l.contains("let _ =") && (l.contains('?') || l.contains("await") || l.contains("()")) {
        return Some((Lens::Failure, "discarded Result with `let _ =` — error path swallowed".into()));
    }
    // malformed: unwrap/expect on external input → panic on hostile input.
    if (l.contains(".unwrap()") || l.contains(".expect(")) && !l.contains("#[") {
        return Some((Lens::Malformed, "unwrap/expect — panics on malformed/unexpected input".into()));
    }
    // boundary: raw index/slice → panic on empty/oob.
    if l.contains("[0]") || l.contains(".first().unwrap") {
        return Some((Lens::Boundary, "direct index/[0] — panics on empty/out-of-bounds".into()));
    }
    // replay: an upsert/insert without an idempotency-key comment nearby.
    if (l.contains("INSERT") || l.contains(".insert(")) && !l.contains("upsert") {
        return Some((Lens::Replay, "insert without visible idempotency key — re-run may duplicate".into()));
    }
    // concurrency: a check-then-act on shared state (read followed by write hint).
    if l.contains("if !") && (l.contains("contains") || l.contains("exists")) {
        return Some((Lens::Concurrency, "check-then-act — TOCTOU window between check and use".into()));
    }
    // authz: a handler/route with no visible authorize/allow call on the line.
    if (l.contains("pub async fn") || l.contains("pub fn")) && l.contains("handler") {
        return Some((Lens::Authz, "handler signature — verify an authorize/allow check guards it".into()));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_discarded_result_as_failure() {
        let f = scan_file("x.rs", "let _ = fallible()?;");
        assert!(f.iter().any(|x| x.lens == Lens::Failure));
    }

    #[test]
    fn flags_unwrap_as_malformed() {
        let f = scan_file("x.rs", "let v = parse(input).unwrap();");
        assert!(f.iter().any(|x| x.lens == Lens::Malformed));
    }

    #[test]
    fn flags_index_zero_as_boundary() {
        let f = scan_file("x.rs", "let head = items[0];");
        assert!(f.iter().any(|x| x.lens == Lens::Boundary));
    }

    #[test]
    fn ignores_comment_lines() {
        // A pattern named only in a comment must NOT be flagged.
        let f = scan_file("x.rs", "// never write let _ = foo()? here");
        assert!(f.is_empty(), "comments are guidance, not code");
    }

    #[test]
    fn clean_line_yields_nothing() {
        let f = scan_file("x.rs", "let sum = a.checked_add(b)?;");
        assert!(f.is_empty());
    }

    #[test]
    fn one_line_yields_at_most_one_finding() {
        // unwrap + index on one line → first match (failure-order) wins, single finding.
        let f = scan_file("x.rs", "let v = items[0].unwrap();");
        assert_eq!(f.len(), 1, "one line, one (lens,site) finding");
    }

    #[test]
    fn stops_at_cfg_test_boundary() {
        // The unwrap below #[cfg(test)] is test code → must NOT be flagged.
        let src = "fn prod() {}\n#[cfg(test)]\nmod t {\n  let v = x.unwrap();\n}\n";
        assert!(scan_file("x.rs", src).is_empty(), "test code excluded");
    }
}
