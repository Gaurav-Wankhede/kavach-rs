#!/usr/bin/env bash
# One-shot authoring of the unified `kavach audit` production module. Uses the
# gate's documented KAVACH_TDD_BYPASS escape for the new-module bootstrap
# (chicken-egg: a brand-new module's tests can only compile-fail, which the
# red-oracle does not record — filed as heal.incident.tdd-red-oracle-compile-fail-not-recorded).
# Every file has its SEPARATE *_test.rs (already authored test-first) wired via #[path].
set -euo pipefail
d="crates/kavach-cli/src/cmd/audit"

cat > "$d/walk.rs" <<'RS'
//! ONE source-file walker for the consolidated audit — adopts the loophole
//! sweep's gold-standard test-exclusion (best of the four pre-merge walkers).
use std::path::{Path, PathBuf};

const SKIP_DIRS: &[&str] = &[".git", "target", "node_modules", "dist", "build", ".venv"];

/// Collect scannable `.rs` sources under `root`, skipping VCS/build/vendor dirs
/// and every test-file convention.
pub(super) fn source_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect(root, &mut out);
    out.sort();
    out
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let skip = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| SKIP_DIRS.contains(&n));
            if !skip {
                collect(&path, out);
            }
        } else if is_scannable_rust(&path.to_string_lossy()) {
            out.push(path);
        }
    }
}

/// True for a non-test Rust source. Excludes `tests.rs`, any `_test` stem
/// segment, and anything under a `tests/` dir.
pub(super) fn is_scannable_rust(path: &str) -> bool {
    let p = Path::new(path);
    if p.extension().is_none_or(|e| !e.eq_ignore_ascii_case("rs")) {
        return false;
    }
    let s = path.replace('\\', "/");
    if s.contains("/tests/") || s.starts_with("tests/") {
        return false;
    }
    p.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| name != "tests.rs" && !name.trim_end_matches(".rs").contains("_test"))
}

#[cfg(test)]
#[path = "walk_test.rs"]
mod walk_test;
RS

cat > "$d/scan.rs" <<'RS'
//! Parallel scan over the selected lenses — scoped-thread sharding, zero-LLM.
use super::finding::Finding;
use super::lens::{self, Selection};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Scan every file across the selected lenses on scoped threads, merged + deduped.
#[must_use]
pub(super) fn scan_all(root: &Path, files: &[PathBuf], sel: Selection) -> Vec<Finding> {
    let out: Mutex<Vec<Finding>> = Mutex::new(Vec::new());
    let workers = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);
    let chunk = files.len().div_ceil(workers).max(1);
    std::thread::scope(|s| {
        for shard in files.chunks(chunk) {
            let out = &out;
            s.spawn(move || {
                let mut local = Vec::new();
                for path in shard {
                    let Ok(src) = std::fs::read_to_string(path) else {
                        continue;
                    };
                    let rel = path.strip_prefix(root).unwrap_or(path);
                    let file = rel.to_string_lossy();
                    local.extend(lens::scan_file(&file, &src, sel));
                }
                if let Ok(mut g) = out.lock() {
                    g.append(&mut local);
                }
            });
        }
    });
    let mut v = out.into_inner().unwrap_or_default();
    v.sort_by_key(Finding::dedup_key);
    v.dedup_by_key(Finding::dedup_key);
    v
}
RS

cat > "$d/report.rs" <<'RS'
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
    for f in findings {
        println!(
            "  [{}] {}:{} {}/{} — {} ({})",
            f.severity.label(),
            f.file,
            f.line,
            f.lens.slug(),
            f.detector,
            f.hint,
            f.fix
        );
    }
    1
}

fn count(findings: &[Finding], sev: Severity) -> usize {
    findings.iter().filter(|f| f.severity == sev).count()
}
RS

cat > "$d/lens/yagni.rs" <<'RS'
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
RS

cat > "$d/lens/worst_practice.rs" <<'RS'
//! Worst-practice "antivirus" lens — shared kavach_patterns detectors, from `cmd/hunt`.
use crate::cmd::audit::finding::{Finding, Lens, Severity};
use kavach_patterns::{owasp_guard, rust_196_guard, rust_guard, silent_io_guard};

/// Run every kavach_patterns detector over one file, mapped into unified findings.
pub(crate) fn scan(file: &str, content: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    for h in silent_io_guard::detect(file, content) {
        out.push(mk("silent_io", file, h.line, Severity::Block, h.category, h.fix));
    }
    for f in owasp_guard::detect(file, content) {
        out.push(mk("owasp", file, f.line, Severity::Block, f.category, f.fix));
    }
    for v in rust_guard::detect(file, content) {
        out.push(mk("rust", file, v.line, Severity::Advisory, &v.pattern, &v.fix));
    }
    for v in rust_196_guard::detect(file, content) {
        use kavach_patterns::rust_196_guard::Rust196Severity as S;
        let sev = match v.severity {
            S::P0Block => Severity::Block,
            S::P1Advisory => Severity::Advisory,
            S::P2Warning => Severity::Warn,
        };
        out.push(mk("rust1.96", file, v.line, sev, v.pattern, v.fix));
    }
    out
}

fn mk(detector: &str, file: &str, line: usize, severity: Severity, hint: &str, fix: &str) -> Finding {
    Finding {
        lens: Lens::WorstPractice,
        detector: detector.to_owned(),
        file: file.to_owned(),
        line,
        severity,
        hint: hint.to_owned(),
        fix: fix.to_owned(),
    }
}

#[cfg(test)]
#[path = "worst_practice_test.rs"]
mod worst_practice_test;
RS

cat > "$d/lens/silent_fail.rs" <<'RS'
//! Silent-failure lens — shared kavach_patterns::silent_io_guard, scoped as the
//! old `kavach doctor` intent. Block severity.
use crate::cmd::audit::finding::{Finding, Lens, Severity};

/// Scan one file for silent-failure patterns via the shared kernel.
pub(crate) fn scan(file: &str, content: &str) -> Vec<Finding> {
    kavach_patterns::silent_io_guard::detect(file, content)
        .into_iter()
        .map(|h| Finding {
            lens: Lens::SilentFail,
            detector: "silent_fail".to_owned(),
            file: file.to_owned(),
            line: h.line,
            severity: Severity::Block,
            hint: h.category.to_owned(),
            fix: h.fix.to_owned(),
        })
        .collect()
}

#[cfg(test)]
#[path = "silent_fail_test.rs"]
mod silent_fail_test;
RS

cat > "$d/lens/security.rs" <<'RS'
//! Security lens — six attack lenses via the shared loophole kernel, from
//! `cmd/loophole/detect.rs`.
use crate::cmd::audit::finding::{Finding, Lens, Severity};

/// Scan one file across the six attack lenses via the shared kernel.
pub(crate) fn scan(file: &str, content: &str) -> Vec<Finding> {
    kavach_patterns::loophole_lens::scan_text(content)
        .into_iter()
        .map(|f| Finding {
            lens: Lens::Security,
            detector: format!("loophole:{}", f.lens.slug()),
            file: file.to_owned(),
            line: f.line,
            severity: Severity::Warn,
            hint: f.hint.to_owned(),
            fix: "root-cause via the named attack lens; fix at source or prove N/A".to_owned(),
        })
        .collect()
}

#[cfg(test)]
#[path = "security_test.rs"]
mod security_test;
RS

cat > "$d/lens/mod.rs" <<'RS'
//! Audit lenses — one detector family per file, all returning unified Findings.
pub(crate) mod security;
pub(crate) mod silent_fail;
pub(crate) mod worst_practice;
pub(crate) mod yagni;

use super::finding::Finding;

/// Which lenses to run. `All` runs every lens (default).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Selection {
    Code,
    SelfAudit,
    Security,
    All,
}

/// Run the selected lenses over one file's content.
#[must_use]
pub(crate) fn scan_file(file: &str, content: &str, sel: Selection) -> Vec<Finding> {
    let mut out = Vec::new();
    if matches!(sel, Selection::Code | Selection::All) {
        out.extend(yagni::scan(file, content));
        out.extend(worst_practice::scan(file, content));
    }
    if matches!(sel, Selection::SelfAudit | Selection::All) {
        out.extend(silent_fail::scan(file, content));
    }
    if matches!(sel, Selection::Security | Selection::All) {
        out.extend(security::scan(file, content));
    }
    out
}
RS

echo "authored: walk scan report lens/{mod,yagni,worst_practice,silent_fail,security}"
