//! Meta-Harness Loophole Loop (M2 sweep). ONE hunt round: emit the iteration
//! YAML to a /tmp working dir (precise per-round targeting), scan the workspace's
//! Rust sources across the six attack lenses, and record each suspected loophole
//! as a self-heal roadmap card via the H1 RPC single-writer path — the durable
//! Kavach-DB record the autonomous loop dispatches for the fix. Kavach DETECTS +
//! RECORDS only; the subscription native agent does every fix. No metered LLM.
//! SOURCE: decision.meta.loophole-loop-extends-goal-yaml · roadmap meta.unit.loophole-sweep-cli.

mod detect;

use crate::cmd::goal::LoopholeIteration;
use crate::cmd::heal::capture_incident;
use crate::cmd::io_safe::{IoExit, into_exit_code, print_or_exit};
use detect::{Finding, scan_file};

/// Cap on findings turned into cards per sweep round — a runaway match count must
/// not flood the board (boundary brake). Excess is reported, not silently dropped.
const MAX_CARDS_PER_ROUND: usize = 50;

/// `kavach loophole sweep` entry. Emits the round's YAML, scans, records cards.
/// Exit 0 on a clean round (finding loopholes is a SUCCESSFUL sweep).
pub(crate) fn run(project: &str, run_id: &str, round: u32) -> i32 {
    match run_inner(project, run_id, round) {
        Ok(()) => 0,
        Err(io) => into_exit_code(io),
    }
}

fn run_inner(project: &str, run_id: &str, round: u32) -> Result<(), IoExit> {
    // 1. Emit the per-iteration YAML to /tmp — the precise unit-of-work target.
    let iter = LoopholeIteration::new(run_id, round, project);
    match iter.emit_tmp() {
        Ok(p) => print_or_exit(&format!("[loophole] round {round} → {}", p.display()))?,
        Err(e) => print_or_exit(&format!("[loophole] WARN: could not emit /tmp YAML: {e}"))?,
    }

    // 2. Scan every tracked Rust source across the six lenses.
    let findings = scan_workspace();
    if findings.is_empty() {
        return print_or_exit("[loophole] round clean: 0 loopholes detected");
    }

    // 3. Record each finding as a heal card (the durable DB record + dispatch unit).
    //    Bounded; idempotent per (lens, site) via the deterministic incident key.
    let mut recorded = 0_usize;
    for f in findings.iter().take(MAX_CARDS_PER_ROUND) {
        let incident = incident_key(f);
        let summary = format!("loophole[{}]: {} ({}:{})", f.lens.slug(), f.hint, f.file, f.line);
        let body = card_body(f);
        let code = capture_incident(project, &incident, &summary, &body, "HEAD~1");
        if code == 0 {
            recorded = recorded.saturating_add(1);
        } else {
            print_or_exit(&format!("[loophole] WARN: card write returned {code} for {incident}"))?;
        }
    }
    let total = findings.len();
    if total > MAX_CARDS_PER_ROUND {
        // No silent cap: name what was dropped this round.
        print_or_exit(&format!(
            "[loophole] NOTE: {total} findings, capped at {MAX_CARDS_PER_ROUND}; rerun next round for the rest"
        ))?;
    }
    print_or_exit(&format!("[loophole] round {round}: {recorded} loophole card(s) recorded"))
}

/// Deterministic incident key — idempotent per (lens, file, line). Re-sweeping
/// the same loophole UPDATES one card (replay loophole of the loophole loop
/// itself, closed). Slashes/dots in the path are flattened to a safe key token.
fn incident_key(f: &Finding) -> String {
    let safe = f.file.replace(['/', '.', '\\'], "-");
    format!("loophole-{}-{safe}-L{}", f.lens.slug(), f.line)
}

/// Card body carrying the lens, site, hint, and the fix contract.
fn card_body(f: &Finding) -> String {
    format!(
        "[LOOPHOLE]\nlens: {}\nfile: {}\nline: {}\nhint: {}\n\n[FIX_CONTRACT]\n\
         This is a SUSPECTED loophole (heuristic hint, not proof). Root-cause it via the\n\
         {} attack lens, fix AT THE SOURCE or prove N/A with a file:line citation, then\n\
         3-witness verify. Record the verdict; never silence the lint.\n",
        f.lens.slug(), f.file, f.line, f.hint, f.lens.slug()
    )
}

/// Scan every `.rs` file under `crates/` (tracked sources), collecting findings.
/// A file that cannot be read is skipped (never aborts the sweep).
fn scan_workspace() -> Vec<Finding> {
    let mut out = Vec::new();
    for path in rust_sources() {
        if let Ok(src) = std::fs::read_to_string(&path) {
            out.extend(scan_file(&path, &src));
        }
    }
    out
}

/// Repo-relative `.rs` paths under `crates/`, via `git ls-files` (tracked only —
/// never scans target/ or vendored build output). Empty on any git error.
fn rust_sources() -> Vec<String> {
    let Ok(out) = std::process::Command::new("git")
        .args(["ls-files", "crates/*.rs", "crates/**/*.rs"])
        .output()
    else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && is_scannable_rust(l))
        .map(str::to_owned)
        .collect()
}

/// True for a non-test Rust source. Test code (tests.rs, `*_test.rs`, anything
/// under a `tests/` dir) is excluded — the workspace lint contract already
/// tolerates `unwrap`/`expect` there, so flagging it floods the board with
/// non-defects (the noise loophole that surfaced on the first real sweep).
fn is_scannable_rust(path: &str) -> bool {
    let p = std::path::Path::new(path);
    if p.extension().is_none_or(|e| !e.eq_ignore_ascii_case("rs")) {
        return false;
    }
    let s = path.replace('\\', "/");
    if s.contains("/tests/") || s.starts_with("tests/") {
        return false;
    }
    p.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| name != "tests.rs" && !name.ends_with("_test.rs"))
}
