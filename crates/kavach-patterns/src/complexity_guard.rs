// split: intentional — guard module, not handler
//! Lightweight complexity guard — LOC, nesting depth, function count.

#[derive(Debug)]
#[non_exhaustive]
pub struct ComplexityReport {
    pub total_lines: usize,
    pub max_nesting: usize,
    pub fn_count: usize,
}

const MAX_LOC: usize = 100;
const MAX_NESTING: usize = 6;
const MAX_FNS: usize = 15;

/// Analyze code complexity from content string.
#[must_use]
pub fn analyze(content: &str) -> ComplexityReport {
    let (mut max_n, mut cur_n, mut fn_c) = (0usize, 0usize, 0usize);
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with("pub fn ")
            || t.starts_with("fn ")
            || t.starts_with("pub async fn ")
            || t.starts_with("async fn ")
            || t.starts_with("pub(crate) fn ")
            || t.starts_with("pub(crate) async fn ")
        {
            fn_c = fn_c.saturating_add(1);
        }
        for ch in t.chars() {
            match ch {
                '{' => {
                    cur_n = cur_n.saturating_add(1);
                    if cur_n > max_n {
                        max_n = cur_n;
                    }
                }
                '}' => {
                    cur_n = cur_n.saturating_sub(1);
                }
                _ => {}
            }
        }
    }
    ComplexityReport {
        total_lines: content.lines().count(),
        max_nesting: max_n,
        fn_count: fn_c,
    }
}

/// Check file complexity. Returns block message if thresholds exceeded.
#[must_use]
pub fn check(file_path: &str, content: &str) -> Option<String> {
    if content.is_empty() || crate::is_test_file(file_path) || !crate::is_code_file(file_path) {
        return None;
    }

    let r = analyze(content);
    let mut issues = Vec::new();
    if r.total_lines > MAX_LOC {
        issues.push(format!("LOC={} > {MAX_LOC}", r.total_lines));
    }
    if r.max_nesting > MAX_NESTING {
        issues.push(format!("nesting={} > {MAX_NESTING}", r.max_nesting));
    }
    if r.fn_count > MAX_FNS {
        issues.push(format!("fns={} > {MAX_FNS}", r.fn_count));
    }
    if issues.is_empty() {
        return None;
    }
    let mut msg = format!("BOUNTY_COMPLEXITY_BLOCK: {file_path}\n");
    for i in &issues {
        use std::fmt::Write;
        let _ = writeln!(msg, "  {i}").ok();
    }
    Some(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_file_passes() {
        assert!(check("src/main.rs", "fn main() {\n    let x = 1;\n}\n").is_none());
    }

    #[test]
    fn deep_nesting_detected() {
        let mut c = String::from("fn f() {\n");
        for _ in 0..7 {
            c.push_str("  if true {\n");
        }
        c.push_str("    let x = 1;\n");
        for _ in 0..7 {
            c.push_str("  }\n");
        }
        c.push_str("}\n");
        assert!(check("src/deep.rs", &c).is_some());
    }

    #[test]
    fn skips_test_files() {
        let c = "let x = 1;\n".repeat(210);
        assert!(check("src/tests/big.rs", &c).is_none());
    }

    #[test]
    fn counts_functions() {
        use std::fmt::Write;
        let mut c = String::with_capacity(300);
        for i in 0..20 {
            writeln!(c, "fn f{i}() {{}}").ok();
        }
        assert_eq!(analyze(&c).fn_count, 20);
    }
}
