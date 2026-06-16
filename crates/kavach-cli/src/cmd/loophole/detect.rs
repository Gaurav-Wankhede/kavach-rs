//! Per-file loophole detection for the CLI sweep — a thin adapter over the shared
//! `kavach_patterns::loophole_lens` kernel (the ONE source of truth for the lens
//! heuristics, also used by the engine Stop-gate hook). This layer only attaches
//! the file path to each kernel finding; the heuristics themselves live in
//! patterns so the sweep and the gate can never drift apart.
//! Kavach DETECTS + RECORDS; it never calls an LLM.
//! SOURCE: decision.meta.loophole-loop-extends-goal-yaml · CLAUDE.md `loophole_self_interrogation`.

use crate::cmd::goal::Lens;

/// One suspected loophole: which lens, which file, which 1-based line, and a hint.
pub(super) struct Finding {
    pub lens: Lens,
    pub file: String,
    pub line: usize,
    pub hint: String,
}

/// Run every lens over `source` (a Rust file's text at `path`) via the shared
/// kernel and attach the file path to each finding.
pub(super) fn scan_file(path: &str, source: &str) -> Vec<Finding> {
    kavach_patterns::loophole_lens::scan_text(source)
        .into_iter()
        .map(|f| Finding {
            lens: Lens::from_kernel(f.lens),
            file: path.to_owned(),
            line: f.line,
            hint: f.hint.to_owned(),
        })
        .collect()
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
    fn attaches_path_and_one_based_line() {
        let f = scan_file("crates/x/src/y.rs", "fn ok() {}\nlet v = items[0];");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].file, "crates/x/src/y.rs");
        assert_eq!(f[0].line, 2, "1-based line carried from the kernel");
        assert_eq!(f[0].lens, Lens::Boundary);
    }

    #[test]
    fn stops_at_cfg_test_boundary() {
        let src = "fn prod() {}\n#[cfg(test)]\nmod t {\n  let v = x.unwrap();\n}\n";
        assert!(scan_file("x.rs", src).is_empty(), "test code excluded");
    }
}
