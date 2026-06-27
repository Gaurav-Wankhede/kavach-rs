//! Detector registry — adapts each kavach-patterns detector into `Finding`s.

use super::finding::{Finding, Severity};
use kavach_patterns::{owasp_guard, rust_guard, silent_io_guard};

/// Run every registered detector over one file's content, returning all hits.
#[must_use]
pub fn scan_file(path: &str, content: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    out.extend(silent_io(path, content));
    out.extend(owasp(path, content));
    out.extend(rust(path, content));
    out
}

fn silent_io(path: &str, content: &str) -> Vec<Finding> {
    silent_io_guard::detect(path, content)
        .into_iter()
        .map(|h| Finding {
            detector: "silent_io",
            file: path.to_owned(),
            line: h.line,
            severity: Severity::Block,
            category: h.category.to_owned(),
            snippet: h.matched.trim().to_owned(),
            fix: h.fix.to_owned(),
        })
        .collect()
}

fn owasp(path: &str, content: &str) -> Vec<Finding> {
    owasp_guard::detect(path, content)
        .into_iter()
        .map(|f| Finding {
            detector: "owasp",
            file: path.to_owned(),
            line: f.line,
            severity: Severity::Block,
            category: f.category.to_owned(),
            snippet: f.pattern,
            fix: f.fix.to_owned(),
        })
        .collect()
}

fn rust(path: &str, content: &str) -> Vec<Finding> {
    rust_guard::detect(path, content)
        .into_iter()
        .map(|v| Finding {
            detector: "rust",
            file: path.to_owned(),
            line: v.line,
            severity: Severity::Advisory,
            category: v.pattern.clone(),
            snippet: v.pattern,
            fix: v.fix,
        })
        .collect()
}
