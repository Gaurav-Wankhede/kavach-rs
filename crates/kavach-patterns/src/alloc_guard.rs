// split: intentional — guard module, not handler
//! Allocation guard — detect allocation-heavy patterns in hot paths.

use regex::Regex;
use std::sync::LazyLock;

struct AllocPattern {
    re: Regex,
    category: &'static str,
    fix: &'static str,
}

fn mk(pat: &str, cat: &'static str, fix: &'static str) -> Option<AllocPattern> {
    Regex::new(pat).map_or_else(
        |_| None,
        |re| {
            Some(AllocPattern {
                re,
                category: cat,
                fix,
            })
        },
    )
}

static PATTERNS: LazyLock<Vec<AllocPattern>> = LazyLock::new(|| {
    vec![
        mk(
            r"\.collect\(\)\s*;",
            "Unbounded collect",
            "Add .take(LIMIT) or use with_capacity()",
        ),
        mk(
            r"String::from\(",
            "String alloc in potential hot path",
            "Consider &str or Cow<str> if borrowed lifetime works",
        ),
        mk(
            r"\.to_string\(\)\s*[,;)]",
            "to_string alloc",
            "Consider &str reference or write! macro",
        ),
        mk(
            r"format!\s*\(",
            "format! allocation",
            "Consider write! to pre-allocated buffer in loops",
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
});

/// Advisory for allocation patterns. Returns None if clean.
pub fn advise(file_path: &str, content: &str) -> Option<String> {
    if content.is_empty() || crate::is_test_file(file_path) {
        return None;
    }
    if !crate::is_code_file(file_path) {
        return None;
    }

    let mut findings = Vec::new();
    for (i, line) in content.lines().enumerate() {
        for p in PATTERNS.iter() {
            if p.re.is_match(line) {
                findings.push(format!(
                    "  L{}: {} — {}",
                    i.saturating_add(1),
                    p.category,
                    p.fix
                ));
                break; // one finding per line
            }
        }
    }
    // Only report if many alloc patterns (>5 = hot path smell)
    if findings.len() < 5 {
        return None;
    }
    let mut msg = format!(
        "[ALLOC_ADVISORY] {} allocation patterns in {file_path}:\n",
        findings.len()
    );
    for f in findings.iter().take(10) {
        msg.push_str(f);
        msg.push('\n');
    }
    Some(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn few_allocs_pass() {
        let code = "let s = String::from(\"hello\");\n";
        assert!(advise("src/main.rs", code).is_none());
    }

    #[test]
    fn many_allocs_flagged() {
        use std::fmt::Write;
        let mut code = String::with_capacity(500);
        for i in 0..10 {
            writeln!(code, "let s{i} = String::from(\"val\");").ok();
        }
        assert!(advise("src/hot.rs", &code).is_some());
    }

    #[test]
    fn skips_tests() {
        use std::fmt::Write;
        let mut code = String::with_capacity(500);
        for i in 0..10 {
            writeln!(code, "let s{i} = String::from(\"val\");").ok();
        }
        assert!(advise("src/tests/t.rs", &code).is_none());
    }
}
