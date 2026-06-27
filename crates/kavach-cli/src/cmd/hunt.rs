//! `kavach hunt [path]` — deterministic antivirus-for-code (zero-LLM regex sweep).

mod finding;
mod registry;
mod scan;

use finding::{Finding, Severity};
use std::path::{Path, PathBuf};

/// `kavach hunt` entry. Exit: 0 = clean, 1 = findings, 2 = unreadable root.
#[must_use]
pub(crate) fn run(root: &Path) -> i32 {
    if !root.exists() {
        eprintln!("hunt: target path missing: {}", root.display());
        return 2;
    }
    let files = source_files(root);
    let findings = scan::scan_parallel(root, &files);
    report(&findings, files.len());
    i32::from(!findings.is_empty())
}

/// Collect scannable source files under `root`, skipping VCS/build/vendor dirs.
fn source_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect(root, &mut out);
    out
}

const SKIP_DIRS: &[&str] = &[".git", "target", "node_modules", "dist", "build", ".venv"];

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(dir_iter) = std::fs::read_dir(dir) else {
        return;
    };
    for item in dir_iter.flatten() {
        let path = item.path();
        if path.is_dir() {
            let skip = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| SKIP_DIRS.contains(&n));
            if !skip {
                collect(&path, out);
            }
        } else if kavach_patterns::is_code_file(&path.to_string_lossy()) {
            out.push(path);
        }
    }
}

/// Print a grouped antivirus report: total scanned, count by severity, each site.
fn report(findings: &[Finding], files_scanned: usize) {
    println!("[KAVACH_HUNT] scanned {files_scanned} source file(s)");
    if findings.is_empty() {
        println!("  result: CLEAN — no worst-practice signatures matched");
        return;
    }
    let block = count(findings, Severity::Block);
    let warn = count(findings, Severity::Warn);
    let adv = count(findings, Severity::Advisory);
    println!(
        "  result: {} finding(s) — {block} BLOCK · {warn} WARN · {adv} ADVISORY",
        findings.len()
    );
    for f in findings {
        let snip = if f.snippet.is_empty() {
            String::new()
        } else {
            format!(" `{}`", f.snippet)
        };
        println!(
            "  [{}] {}:{} {}/{}{snip} — {}",
            f.severity.label(),
            f.file,
            f.line,
            f.detector,
            f.category,
            f.fix
        );
    }
}

fn count(findings: &[Finding], sev: Severity) -> usize {
    findings.iter().filter(|f| f.severity == sev).count()
}

#[cfg(test)]
#[path = "hunt_test.rs"]
mod hunt_test;
