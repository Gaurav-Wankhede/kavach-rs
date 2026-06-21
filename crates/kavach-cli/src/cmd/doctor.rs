//! `kavach doctor` — read-only self-audit of kavach's OWN source.
//!
//! Kavach instruments the AGENT's behavior but historically not ITSELF: silent
//! self-writes, write/read split-brain, and destructive self-mutation went
//! undetected until they bit (see decision.kavach-self-watchdog-design). This is
//! Layer 3 of that design — kavach running a silent-failure audit ON kavach.
//!
//! It scans the engine/session/rpc/surreal crate sources for four classes and
//! PRINTS findings (read-only — no DB writes, human triage per the design). Exit
//! code is non-zero when findings exist so it is usable as a CI gate.

mod detect;

use detect::{Finding, scan_source};
use std::path::Path;

/// Crate `src/` roots that constitute kavach's own governed code. A finding here
/// is a kavach self-bug; code outside these dirs is a user project, not our scope.
const SELF_CRATES: [&str; 4] = [
    "crates/kavach-engine/src",
    "crates/kavach-session/src",
    "crates/kavach-rpc/src",
    "crates/kavach-surreal/src",
];

/// `kavach doctor` entry. Walks the self-crate sources, runs the audit matrix,
/// prints a grouped report. Returns exit code: 0 = clean, 1 = findings, 2 = the
/// audit could not run (workspace root not found) — never a silent success.
#[must_use]
pub(crate) fn run(workspace_root: &Path) -> i32 {
    let mut findings: Vec<Finding> = Vec::new();
    let mut files_scanned = 0usize;
    for rel in SELF_CRATES {
        let root = workspace_root.join(rel);
        if !root.exists() {
            eprintln!("doctor: self-crate path missing: {}", root.display());
            return 2;
        }
        for path in rust_files(&root) {
            let Ok(src) = std::fs::read_to_string(&path) else {
                continue;
            };
            files_scanned = files_scanned.saturating_add(1);
            let rel_path = path.strip_prefix(workspace_root).unwrap_or(&path);
            findings.extend(scan_source(&rel_path.to_string_lossy(), &src));
        }
    }
    report(&findings, files_scanned);
    i32::from(!findings.is_empty())
}

/// Collect every `.rs` file under `root` (recursive, test files included — a
/// silent-fail in a test helper still hides real failures).
fn rust_files(root: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    collect(root, &mut out);
    out.sort();
    out
}

fn collect(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Print a grouped report: count per class, then each finding as `file:line`.
fn report(findings: &[Finding], files_scanned: usize) {
    println!("[KAVACH_DOCTOR] self-audit over {files_scanned} files in kavach's own crates");
    if findings.is_empty() {
        println!("  result: CLEAN — no silent-fail / unproven-DELETE patterns found");
        return;
    }
    println!("  result: {} finding(s) — review (read-only; not auto-filed):", findings.len());
    for f in findings {
        println!("  [{}] {}:{} — {}", f.class.label(), f.file, f.line, f.hint);
    }
    println!(
        "  next: triage each (real silent-fail -> fix at root; benign-by-design -> add a // doctor:ok comment)"
    );
}

#[cfg(test)]
#[path = "doctor_test.rs"]
mod doctor_test;
