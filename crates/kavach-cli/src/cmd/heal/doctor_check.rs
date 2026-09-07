//! `kavach doctor` logic — read-only self-audit of kavach's OWN source.
//!
//! Kavach instruments the AGENT's behavior but historically not ITSELF: silent
//! self-writes, write/read split-brain, and destructive self-mutation went
//! undetected until they bit (see decision.kavach-self-watchdog-design). This is
//! Layer 3 of that design — kavach running a silent-failure audit ON kavach.
//!
//! It scans the engine/session/rpc/surreal crate sources for four classes and
//! returns findings (read-only — no DB writes, human triage per the design).

pub(crate) mod detect;

use detect::{Finding, scan_source};
use std::path::Path;

/// Crate `src/` roots that constitute kavach's own governed code. A finding here
/// is a kavach self-bug; code outside these dirs is a user project, not our scope.
const SELF_CRATES: [&str; 5] = [
    "crates/kavach-engine/src",
    "crates/kavach-session/src",
    "crates/kavach-rpc/src",
    "crates/kavach-db/src",
    "crates/kavach-surreal/src",
];

/// Run the kavach self-audit (doctor check). Returns findings found.
pub(crate) fn run_doctor(workspace_root: &Path) -> Vec<Finding> {
    let mut findings: Vec<Finding> = Vec::new();
    for rel in SELF_CRATES {
        let root = workspace_root.join(rel);
        if !root.exists() {
            continue;
        }
        for path in rust_files(&root) {
            let Ok(src) = std::fs::read_to_string(&path) else {
                continue;
            };
            let rel_path = path.strip_prefix(workspace_root).unwrap_or(&path);
            findings.extend(scan_source(&rel_path.to_string_lossy(), &src));
        }
    }
    findings
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doctor_finds_silent_fail() {
        let src = r#"
fn foo() {
    let _ = db.query("SELECT 1"); // silent failure - result not checked
}
"#;
        let findings = scan_source("test.rs", src);
        assert!(!findings.is_empty());
    }
}