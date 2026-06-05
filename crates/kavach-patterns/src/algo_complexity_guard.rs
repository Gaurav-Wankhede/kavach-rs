// split: intentional — guard module, not handler
//! Algorithm complexity guard — detect O(n²) patterns, nested loops, unbounded collect.

use regex::Regex;
use std::fmt::Write;
use std::sync::LazyLock;

struct AlgoPattern {
    re: Regex,
    category: &'static str,
    fix: &'static str,
}

fn mk(pat: &str, cat: &'static str, fix: &'static str) -> Option<AlgoPattern> {
    Regex::new(pat).map_or_else(
        |_| None,
        |re| {
            Some(AlgoPattern {
                re,
                category: cat,
                fix,
            })
        },
    )
}

static PATTERNS: LazyLock<Vec<AlgoPattern>> = LazyLock::new(|| {
    vec![
        mk(
            r"for .+ in .+\{[\s\S]*?for .+ in .+\{",
            "O(n²) nested loops",
            "Consider HashMap/HashSet for O(1) lookup or Iterator combinators",
        ),
        mk(
            r"\.contains\(.+\)\s*\{",
            "Linear search in loop",
            "Use HashSet for O(1) lookup instead of Vec::contains()",
        ),
        mk(
            r"\.collect::<Vec",
            "Unbounded collect",
            "Add .take(LIMIT) or pre-check size before collecting",
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
});

#[derive(Debug)]
#[non_exhaustive]
pub struct AlgoFinding {
    pub category: &'static str,
    pub fix: &'static str,
    pub line: usize,
}

/// Scan for algorithm complexity issues. Returns advisories (not blocks).
pub fn detect(file_path: &str, content: &str) -> Vec<AlgoFinding> {
    if content.is_empty() || crate::is_test_file(file_path) {
        return vec![];
    }
    let mut findings = Vec::new();
    for (i, line) in content.lines().enumerate() {
        for p in PATTERNS.iter() {
            if p.re.is_match(line) {
                findings.push(AlgoFinding {
                    category: p.category,
                    fix: p.fix,
                    line: i.saturating_add(1),
                });
            }
        }
    }
    findings
}

/// Advisory message (not block) for algorithm complexity.
#[must_use]
pub fn advise(file_path: &str, content: &str) -> Option<String> {
    let findings = detect(file_path, content);
    if findings.is_empty() {
        return None;
    }
    let mut msg = String::from("[ALGO_COMPLEXITY_ADVISORY]\n");
    for f in &findings {
        writeln!(msg, "  L{}: {} — {}", f.line, f.category, f.fix).ok();
    }
    Some(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_contains_in_loop() {
        let code = "if items.contains(&x) {\n    do_thing();\n}\n";
        let f = detect("src/h.rs", code);
        assert!(!f.is_empty());
    }

    #[test]
    fn clean_code_passes() {
        let code = "let x = map.get(&key);\n";
        assert!(detect("src/h.rs", code).is_empty());
    }

    #[test]
    fn skips_tests() {
        let code = "if items.contains(&x) {\n";
        assert!(detect("src/tests/t.rs", code).is_empty());
    }
}
