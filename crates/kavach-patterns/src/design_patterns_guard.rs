// split: intentional — single guard module for design-pattern advisories
//! Rust Design Patterns Guard — coverage for the rust-unofficial catalog
//! gaps not detected by existing guards.
//!
//! SOURCES (verified 2026-05):
//! - <https://rust-unofficial.github.io/patterns>/
//! - <https://rust-unofficial.github.io/patterns/anti_patterns/deny-warnings.html>
//! - <https://rust-unofficial.github.io/patterns/anti_patterns/deref.html>
//! - <https://rust-unofficial.github.io/patterns/idioms/coercion-arguments.html>
//! - <https://rust-unofficial.github.io/patterns/patterns/creational/builder.html>

use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PatternSeverity {
    P1Advisory,
    P2Warning,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PatternViolation {
    pub severity: PatternSeverity,
    pub pattern: &'static str,
    pub fix: &'static str,
    pub line: usize,
}

struct Rule {
    // `Option` so the `LazyLock` initializer never needs unwrap/expect (both
    // `forbid` at the workspace level). `None` is unreachable for these const
    // patterns; a `None` rule is simply skipped at match time.
    re: &'static Option<Regex>,
    sev: PatternSeverity,
    pattern: &'static str,
    fix: &'static str,
}

static RE_DENY_WARNINGS: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"#!?\[deny\(\s*warnings\s*\)\]").ok());

static RE_STRING_PARAM: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"\bfn\s+\w+\s*\([^)]*&\s*String\b").ok());

static RE_VEC_PARAM: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"\bfn\s+\w+\s*\([^)]*&\s*Vec\s*<").ok());

static RE_BOX_PARAM: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"\bfn\s+\w+\s*\([^)]*&\s*Box\s*<").ok());

static RE_DEREF_IMPL: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"impl\s+(?:std::ops::)?Deref\s+for\s+\w+\s*\{").ok());

static RULES: LazyLock<Vec<Rule>> = LazyLock::new(build_rules);

fn build_rules() -> Vec<Rule> {
    vec![
        Rule {
            re: &RE_DENY_WARNINGS,
            sev: PatternSeverity::P1Advisory,
            pattern: "deny(warnings) anti-pattern",
            fix: "Use RUSTFLAGS=\"-D warnings\" in CI or deny specific lints.",
        },
        Rule {
            re: &RE_STRING_PARAM,
            sev: PatternSeverity::P1Advisory,
            pattern: "borrowed-owned param: &String",
            fix: "Use &str — accepts both String and string literals via deref coercion.",
        },
        Rule {
            re: &RE_VEC_PARAM,
            sev: PatternSeverity::P1Advisory,
            pattern: "borrowed-owned param: &Vec<T>",
            fix: "Use &[T] — accepts Vec, arrays, and slices via deref coercion.",
        },
        Rule {
            re: &RE_BOX_PARAM,
            sev: PatternSeverity::P1Advisory,
            pattern: "borrowed-owned param: &Box<T>",
            fix: "Use &T — Box already provides indirection, double-borrow is wasteful.",
        },
        Rule {
            re: &RE_DEREF_IMPL,
            sev: PatternSeverity::P2Warning,
            pattern: "impl Deref — verify pointer-like semantics",
            fix: "Deref is for smart pointers. If faking inheritance, use traits + composition.",
        },
    ]
}

/// Scan content for Rust design pattern advisories.
pub fn detect(file_path: &str, content: &str) -> Vec<PatternViolation> {
    if content.is_empty() || crate::file_types::is_test_file(file_path) {
        return vec![];
    }
    if !std::path::Path::new(file_path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("rs"))
    {
        return vec![];
    }

    let mut violations = Vec::with_capacity(4);

    for (i, line) in content.lines().enumerate() {
        for rule in RULES.iter() {
            if rule.re.as_ref().is_some_and(|re| re.is_match(line)) {
                violations.push(PatternViolation {
                    severity: rule.sev,
                    pattern: rule.pattern,
                    fix: rule.fix,
                    line: i.saturating_add(1),
                });
            }
        }
    }

    if let Some(line_no) = detect_many_arg_constructor(content) {
        violations.push(PatternViolation {
            severity: PatternSeverity::P1Advisory,
            pattern: "many-arg constructor without Builder",
            fix: "Constructor with >4 args is hard to call. Add Builder pattern.",
            line: line_no,
        });
    }

    violations
}

static RE_NEW_CONSTRUCTOR: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"fn\s+new\s*\(([^)]*)\)").ok());

fn detect_many_arg_constructor(content: &str) -> Option<usize> {
    let re = RE_NEW_CONSTRUCTOR.as_ref()?;
    for m in re.captures_iter(content) {
        let Some(args_match) = m.get(1) else { continue };
        let args = args_match.as_str();
        let comma_count = args.matches(',').count();
        if comma_count >= 4 {
            let Some(full) = m.get(0) else { continue };
            let line_no = content
                .get(..full.start())
                .map_or(0, |s| s.matches('\n').count())
                .saturating_add(1);
            return Some(line_no);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_deny_warnings() {
        let code = "#![deny(warnings)]\nfn main() {}";
        let v = detect("src/lib.rs", code);
        assert!(v.iter().any(|x| x.pattern.contains("deny(warnings)")));
    }

    #[test]
    fn detects_string_param() {
        let code = "fn parse(s: &String) -> usize { s.len() }";
        let v = detect("src/util.rs", code);
        assert!(v.iter().any(|x| x.pattern.contains("&String")));
    }

    #[test]
    fn detects_vec_param() {
        let code = "fn sum(v: &Vec<i32>) -> i32 { v.iter().sum() }";
        let v = detect("src/util.rs", code);
        assert!(v.iter().any(|x| x.pattern.contains("&Vec")));
    }

    #[test]
    fn detects_many_arg_new() {
        let code =
            "impl Foo { pub fn new(a: i32, b: i32, c: i32, d: i32, e: i32) -> Self { Self {} } }";
        let v = detect("src/foo.rs", code);
        assert!(v.iter().any(|x| x.pattern.contains("Builder")));
    }

    #[test]
    fn allows_str_param() {
        let code = "fn parse(s: &str) -> usize { s.len() }";
        let v = detect("src/util.rs", code);
        assert!(!v.iter().any(|x| x.pattern.contains("&String")));
    }

    #[test]
    fn skips_test_files() {
        let code = "#![deny(warnings)]";
        let v = detect("src/tests/mod.rs", code);
        assert!(v.is_empty());
    }

    #[test]
    fn detects_string_param_multiarg() {
        // Edge case: &String in second position should still match
        let code = "fn parse(s: &str, t: &String) -> usize { s.len() + t.len() }";
        let v = detect("src/util.rs", code);
        assert!(v.iter().any(|x| x.pattern.contains("&String")));
    }

    #[test]
    fn rejects_deref_on_non_smartpointer() {
        // impl Deref is matched regardless of whether it's a smart pointer
        // This is acceptable as P2 (advisory) — false positives are okay for heuristics
        let code = "impl Deref for MyNotAPtr { type Target = Inner; }";
        let v = detect("src/lib.rs", code);
        assert!(v.iter().any(|x| x.pattern.contains("impl Deref")));
    }

    #[test]
    fn rejects_four_comma_constructor() {
        // 4 commas = 5 args; should trigger Builder advisory
        let code = "fn new(a: i32, b: i32, c: i32, d: i32, e: i32) -> Self { Self {} }";
        let v = detect("src/foo.rs", code);
        assert!(v.iter().any(|x| x.pattern.contains("Builder")));
    }

    #[test]
    fn allows_three_arg_constructor() {
        // 3 args (2 commas); should NOT trigger
        let code = "fn new(a: i32, b: i32, c: i32) -> Self { Self {} }";
        let v = detect("src/foo.rs", code);
        assert!(!v.iter().any(|x| x.pattern.contains("Builder")));
    }
}
