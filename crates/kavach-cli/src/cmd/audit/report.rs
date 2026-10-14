//! Grouped audit report + CI exit code (0 clean · 1 findings).
use super::finding::{Finding, Severity};

/// Print a grouped report; return exit code (0 clean, 1 findings).
#[must_use]
pub(super) fn report(findings: &[Finding], files_scanned: usize) -> i32 {
    println!("[KAVACH_AUDIT] scanned {files_scanned} source file(s)");
    if findings.is_empty() {
        println!("  result: CLEAN — no findings across the selected lenses");
        return 0;
    }
    let block = count(findings, Severity::Block);
    let warn = count(findings, Severity::Warn);
    let adv = count(findings, Severity::Advisory);
    println!(
        "  result: {} finding(s) — {block} BLOCK · {warn} WARN · {adv} ADVISORY",
        findings.len()
    );
    let entries: Vec<(&str, Option<String>)> = findings
        .iter()
        .map(|f| {
            let loc = if f.column > 0 {
                format!("L{}:{}", f.line, f.column)
            } else {
                format!("L{}", f.line)
            };
            let status = format!(
                "{} [{}:{}] {} ({})",
                loc,
                f.severity.label(),
                f.detector,
                f.hint,
                f.fix
            );
            (f.file.as_str(), Some(status))
        })
        .collect();
    let tree = kavach_toon::encode_path_tree(entries.iter().map(|(p, s)| (*p, s.as_deref())));
    print!("{}", tree);
    1
}

fn count(findings: &[Finding], sev: Severity) -> usize {
    findings.iter().filter(|f| f.severity == sev).count()
}
