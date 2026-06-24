//! Flags comment BLOAT, language-agnostically — not mere comment count.
//!
//! Policy (§`comments_not_the_deliverable`): concise + precise comments are GOOD;
//! a one-line "why" never fires. What fires is BLOAT — a long rationale paragraph
//! that belongs in the Kavach DB (injectable as decision context), or an
//! over-length wall-of-text line. Signal = prose VOLUME, not line count: a run is
//! noise only when it is both long (`BLOAT_RUN`+ lines) AND prose-heavy — either one
//! `PROSE_LINE_MIN`-char line OR `RUN_PROSE_VOLUME` summed across the run (closes the
//! split-into-short-lines bypass) — OR any single comment exceeds `MAX_LEN`. Exempt:
//! doc/header + `SAFETY:`/shebang/directive markers. Non-source files skipped.
use std::fmt::Write as _;

/// One line is enough; this many consecutive comment lines is a wall.
const BLOAT_RUN: usize = 2;
/// A candidate run is bloat if it ALSO carries a prose-heavy line (this many
/// chars+) — proving it's rationale narration, not a column of terse markers.
const PROSE_LINE_MIN: usize = 60;
/// OR the run's CUMULATIVE comment-text volume reaches this — closes the
/// split-to-evade bypass (one paragraph fragmented into many short <PROSE_LINE_MIN
/// lines never trips the per-line trigger, but its summed prose still narrates).
const RUN_PROSE_VOLUME: usize = 200;
/// Any single comment line longer than this is a wall-of-text — flagged alone.
const MAX_LEN: usize = 100;

const PREFIXES: &[&str] = &["///", "//!", "//", "#", "--", ";"];

fn is_exempt(t: &str) -> bool {
    t.starts_with("#!")
        || t.starts_with("#[")
        || t.starts_with("#include")
        || t.starts_with("#define")
        || t.starts_with("#pragma")
        || t.contains("SAFETY:")
        || t.contains("kavach:intentional")
        || t.contains("```")
        || t.starts_with("#region")
        || t.starts_with("#endregion")
}

fn is_line_comment(t: &str) -> bool {
    if is_exempt(t) {
        return false;
    }
    PREFIXES.iter().any(|p| t.starts_with(p))
}

const SOURCE_EXTS: &[&str] = &[
    ".rs", ".py", ".sql", ".sh", ".bash", ".zsh", ".rb", ".lua", ".pl", ".r", ".jl", ".nim",
    ".go", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".c", ".h", ".cpp", ".cc", ".hpp",
    ".cs", ".java", ".kt", ".kts", ".scala", ".swift", ".php", ".dart", ".zig", ".toml",
    ".yaml", ".yml", ".tf", ".hcl",
];

fn is_source(path: &str) -> bool {
    let p = path.to_lowercase();
    SOURCE_EXTS.iter().any(|e| p.ends_with(e))
}

/// Count of flagged bloat lines in `content` (the block trigger).
fn bloat_count(file_path: &str, content: &str) -> usize {
    advise(file_path, content)
        .map_or(0, |m| m.lines().filter(|l| l.trim_start().starts_with('L')).count())
}

/// True when the write INTRODUCES new bloat (new count > old). Pre-existing bloat
/// never blocks, so legacy files stay editable. Empty `old` = a fresh Write.
#[must_use]
pub fn introduces_bloat(file_path: &str, old: &str, new: &str) -> bool {
    bloat_count(file_path, new) > bloat_count(file_path, old)
}

/// Advisory for multi-line comment-noise blocks, any language. `None` if clean.
#[must_use]
pub fn advise(file_path: &str, content: &str) -> Option<String> {
    if content.is_empty() || crate::is_test_file(file_path) || !is_source(file_path) {
        return None;
    }

    let mut blocks = Vec::new();
    let mut long = Vec::new();
    let mut run_start = 0usize;
    let mut run = 0usize;
    let mut run_has_prose = false;
    let mut run_volume = 0usize;
    for (i, line) in content.lines().enumerate() {
        let t = line.trim_start();
        if is_line_comment(t) {
            if run == 0 {
                run_start = i.saturating_add(1);
                run_has_prose = false;
                run_volume = 0;
            }
            run = run.saturating_add(1);
            let len = t.chars().count();
            run_volume = run_volume.saturating_add(len);
            // Prose either as ONE long line, or summed across many short lines
            // (the split-to-evade bypass) — both narrate.
            if len >= PROSE_LINE_MIN || run_volume >= RUN_PROSE_VOLUME {
                run_has_prose = true;
            }
            if len > MAX_LEN {
                long.push(i.saturating_add(1));
            }
        } else {
            // Bloat = a long run that ALSO narrates (prose-heavy line). A short
            // run, or a long column of terse markers, is precise — never fires.
            if run >= BLOAT_RUN && run_has_prose {
                blocks.push((run_start, run));
            }
            run = 0;
        }
    }
    if run >= BLOAT_RUN && run_has_prose {
        blocks.push((run_start, run));
    }
    if blocks.is_empty() && long.is_empty() {
        return None;
    }

    let mut msg = format!(
        "[COMMENT_NOISE] {file_path} — §comments_not_the_deliverable: keep comments \
         concise + precise; move long rationale to the Kavach DB (a decision row, \
         injectable as context) — not a paragraph in the file:\n"
    );
    for (start, len) in blocks.iter().take(10) {
        writeln!(msg, "  L{start}: {len}-line rationale paragraph → DB").ok();
    }
    for line in long.iter().take(10) {
        writeln!(msg, "  L{line}: comment >{MAX_LEN} chars (wall-of-text)").ok();
    }
    Some(msg)
}

#[cfg(test)]
#[path = "comment_noise_block_test.rs"]
mod block_test;

#[cfg(test)]
mod tests {
    use super::advise;

    #[test]
    fn short_precise_block_is_clean() {
        // 3 terse lines are precise, not bloat — must NOT fire (policy change).
        let c = "fn f() {}\n// one\n// two\n// three\nfn g() {}\n";
        assert!(advise("src/x.rs", c).is_none());
    }

    #[test]
    fn long_rationale_paragraph_flagged() {
        // 6+ lines AND prose-heavy (a ≥60-char narration line) = bloat → DB.
        let prose = "x".repeat(70);
        let c = format!(
            "fn f() {{}}\n// {prose}\n// b\n// c\n// d\n// e\n// f\nfn g() {{}}\n"
        );
        assert!(advise("src/x.rs", &c).is_some());
    }

    #[test]
    fn long_run_of_terse_markers_is_clean() {
        // 8 short comment lines (no prose-heavy line) = precise column, not bloat.
        let c = "// a\n// b\n// c\n// d\n// e\n// f\n// g\n// h\nfn f() {}\n";
        assert!(advise("src/x.rs", c).is_none());
    }

    #[test]
    fn split_to_evade_short_lines_now_fires() {
        // The bypass: one rationale paragraph fragmented into 8 short (<60ch)
        // lines — no single line trips PROSE_LINE_MIN, but summed volume narrates.
        let frag = "// rationale fragment about forty chars\n".repeat(8);
        let c = format!("fn f() {{}}\n{frag}fn g() {{}}\n");
        assert!(advise("src/x.rs", &c).is_some());
    }

    #[test]
    fn one_line_comment_is_clean() {
        let c = "// just one\nfn f() {}\n";
        assert!(advise("src/x.rs", c).is_none());
    }

    #[test]
    fn two_line_comment_is_clean() {
        let c = "// one\n// two\nfn f() {}\n";
        assert!(advise("src/x.rs", c).is_none());
    }

    #[test]
    fn module_header_and_safety_exempt() {
        let c = "//! header\n//! more\n//! lines\nfn f() {}\n// SAFETY: a\n// SAFETY: b\n// SAFETY: c\n";
        assert!(advise("src/x.rs", c).is_none());
    }

    #[test]
    fn long_doc_comment_wall_flagged() {
        let prose = "z".repeat(70);
        let c = format!(
            "/// {prose}\n/// b\n/// c\n/// d\n/// e\n/// f\npub fn g() {{}}\n"
        );
        assert!(advise("src/x.rs", &c).is_some());
    }

    #[test]
    fn long_module_doc_wall_flagged() {
        let prose = "q".repeat(70);
        let c = format!(
            "//! {prose}\n//! b\n//! c\n//! d\n//! e\n//! f\nfn f() {{}}\n"
        );
        assert!(advise("src/x.rs", &c).is_some());
    }

    #[test]
    fn short_doc_comment_is_clean() {
        let c = "/// adds two\npub fn add(a: i64, b: i64) -> i64 { a + b }\n";
        assert!(advise("src/x.rs", c).is_none());
    }

    #[test]
    fn safety_run_stays_exempt() {
        let c = "// SAFETY: a\n// SAFETY: b\n// SAFETY: c\n// SAFETY: d\n// SAFETY: e\n// SAFETY: f\nfn f() {}\n";
        assert!(advise("src/x.rs", c).is_none());
    }

    #[test]
    fn non_code_file_clean() {
        let c = "// a\n// b\n// c\n";
        assert!(advise("notes.md", c).is_none());
    }

    #[test]
    fn python_short_hash_block_is_clean() {
        // 3 terse lines = precise, not bloat (policy change).
        let c = "def f():\n    # one\n    # two\n    # three\n    pass\n";
        assert!(advise("x.py", c).is_none());
    }

    #[test]
    fn python_long_prose_paragraph_flagged() {
        let prose = "y".repeat(70);
        let c = format!(
            "def f():\n    # {prose}\n    # b\n    # c\n    # d\n    # e\n    # f\n    pass\n"
        );
        assert!(advise("x.py", &c).is_some());
    }

    #[test]
    fn sql_short_dash_block_is_clean() {
        let c = "-- one\n-- two\n-- three\nSELECT 1;\n";
        assert!(advise("x.sql", c).is_none());
    }

    #[test]
    fn rust_attribute_run_not_flagged() {
        let c = "#[derive(Debug)]\n#[serde(rename = \"x\")]\n#[allow(dead_code)]\nstruct S;\n";
        assert!(advise("x.rs", c).is_none());
    }

    #[test]
    fn long_single_comment_flagged() {
        let long = "x".repeat(120);
        let c = format!("// {long}\nfn f() {{}}\n");
        assert!(advise("x.rs", &c).is_some());
    }

    #[test]
    fn short_single_comment_clean() {
        let c = "// short note\nfn f() {}\n";
        assert!(advise("x.rs", c).is_none());
    }

    #[test]
    fn shebang_plus_directives_not_flagged() {
        let c = "#!/usr/bin/env python\n#include <x>\n#define Y 1\nint main(){}\n";
        assert!(advise("x.c", c).is_none());
    }
}
