//! Flags multi-line line-comment rationale blocks, language-agnostically.
//!
//! A run of `THRESHOLD`+ consecutive line-comments (`//`, `#`, `--`, `;`) is noise
//! per global `CLAUDE.md` §`comments_not_the_deliverable`. Exempt: doc/header and
//! `SAFETY:`/shebang/directive markers. Markdown/text files are skipped upstream.
use std::fmt::Write as _;

const THRESHOLD: usize = 3;
const MAX_LEN: usize = 100;

const PREFIXES: &[&str] = &["///", "//!", "//", "#", "--", ";"];

fn is_exempt(t: &str) -> bool {
    t.starts_with("//!")
        || t.starts_with("///")
        || t.starts_with("#!")
        || t.starts_with("#[")
        || t.starts_with("#include")
        || t.starts_with("#define")
        || t.starts_with("#pragma")
        || t.contains("SAFETY:")
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
    for (i, line) in content.lines().enumerate() {
        let t = line.trim_start();
        if is_line_comment(t) {
            if run == 0 {
                run_start = i.saturating_add(1);
            }
            run = run.saturating_add(1);
            if t.chars().count() > MAX_LEN {
                long.push(i.saturating_add(1));
            }
        } else {
            if run >= THRESHOLD {
                blocks.push((run_start, run));
            }
            run = 0;
        }
    }
    if run >= THRESHOLD {
        blocks.push((run_start, run));
    }
    if blocks.is_empty() && long.is_empty() {
        return None;
    }

    let mut msg = format!(
        "[COMMENT_NOISE] {file_path} — §comments_not_the_deliverable: \
         concise, ≤1 line. Cut or move rationale to the commit:\n"
    );
    for (start, len) in blocks.iter().take(10) {
        writeln!(msg, "  L{start}: {len}-line block").ok();
    }
    for line in long.iter().take(10) {
        writeln!(msg, "  L{line}: comment >{MAX_LEN} chars").ok();
    }
    Some(msg)
}

#[cfg(test)]
mod tests {
    use super::advise;

    #[test]
    fn flags_three_line_block() {
        let c = "fn f() {}\n// one\n// two\n// three\nfn g() {}\n";
        assert!(advise("src/x.rs", c).is_some());
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
    fn non_code_file_clean() {
        let c = "// a\n// b\n// c\n";
        assert!(advise("notes.md", c).is_none());
    }

    #[test]
    fn python_hash_block_flagged() {
        let c = "def f():\n    # one\n    # two\n    # three\n    pass\n";
        assert!(advise("x.py", c).is_some());
    }

    #[test]
    fn sql_dash_block_flagged() {
        let c = "-- one\n-- two\n-- three\nSELECT 1;\n";
        assert!(advise("x.sql", c).is_some());
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
