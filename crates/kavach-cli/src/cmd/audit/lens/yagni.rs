//! YAGNI / over-engineering lens — from `cmd/lint/audit.rs::tag_line`.
use crate::cmd::audit::finding::{Finding, Lens, Severity};

/// Scan one file's content line-by-line for over-engineering signatures.
pub(crate) fn scan(file: &str, content: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    for (i, line) in content.lines().enumerate() {
        if let Some((what, fix)) = tag_line(line) {
            out.push(Finding {
                lens: Lens::Yagni,
                detector: "yagni".to_owned(),
                file: file.to_owned(),
                line: i.saturating_add(1),
                severity: Severity::Advisory,
                hint: what.to_owned(),
                fix: fix.to_owned(),
            });
        }
    }
    out
}

fn tag_line(line: &str) -> Option<(&'static str, &'static str)> {
    let t = line.trim_start();
    if t.starts_with("#[allow(dead_code)]") {
        return Some(("dead-code allow", "remove the code or the allow"));
    }
    if t.contains(".clone().clone()") {
        return Some(("double clone", "one suffices"));
    }
    if t.contains(".iter().cloned().collect::<Vec") || t.contains(".to_vec().clone()") {
        return Some(("needless collect/clone roundtrip", "drop the roundtrip"));
    }
    if t.contains(".unwrap_or_else(|| Vec::new())") || t.contains(".unwrap_or(Vec::new())") {
        return Some(("unwrap_or_else(Vec::new)", "use unwrap_or_default()"));
    }
    if t.contains(".map(|x| x)") || t.contains(".map(|x| x.clone())") {
        return Some(("identity map", "drop it"));
    }
    None
}

#[cfg(test)]
#[path = "yagni_test.rs"]
mod yagni_test;
