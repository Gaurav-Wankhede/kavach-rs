//! Meta-Harness Loophole Loop (M2 sweep). ONE hunt round: emit the iteration
//! YAML to a /tmp working dir (precise per-round targeting), scan the workspace's
//! Rust sources across the six attack lenses, and record each suspected loophole
//! as a self-heal roadmap card via the H1 RPC single-writer path — the durable
//! Kavach-DB record the autonomous loop dispatches for the fix. Kavach DETECTS +
//! RECORDS only; the subscription native agent does every fix. No metered LLM.
//! SOURCE: decision.meta.loophole-loop-extends-goal-yaml · roadmap meta.unit.loophole-sweep-cli.

pub(crate) mod cron;
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
    match sweep_round(project, run_id, round) {
        Ok(_) => 0,
        Err(io) => into_exit_code(io),
    }
}

/// What one sweep round produced. `keys` is the set of distinct (lens,site)
/// incident keys recorded — the loop-until-dry convergence signal: a round that
/// surfaces NO key absent from the accumulated seen-set is a dry round.
struct RoundOutcome {
    /// Distinct incident keys recorded this round (deduped within the round).
    keys: Vec<String>,
    /// Total raw findings before the per-round cap (for the no-silent-cap note).
    total_findings: usize,
}

/// Run ONE hunt round: emit the iteration YAML to /tmp, scan the workspace across
/// the six lenses, and record each suspected loophole as a heal card. Returns the
/// recorded incident keys + raw finding count so a caller can detect convergence.
fn sweep_round(project: &str, run_id: &str, round: u32) -> Result<RoundOutcome, IoExit> {
    // 1. Emit the per-iteration YAML to /tmp — the precise unit-of-work target.
    let iter = LoopholeIteration::new(run_id, round, project);
    match iter.emit_tmp() {
        Ok(p) => print_or_exit(&format!("[loophole] round {round} → {}", p.display()))?,
        Err(e) => print_or_exit(&format!("[loophole] WARN: could not emit /tmp YAML: {e}"))?,
    }

    // 2. Scan every tracked Rust source across the six lenses.
    let findings = scan_workspace();
    if findings.is_empty() {
        print_or_exit("[loophole] round clean: 0 loopholes detected")?;
        return Ok(RoundOutcome {
            keys: Vec::new(),
            total_findings: 0,
        });
    }

    // 3. Record each finding as a heal card (the durable DB record + dispatch unit).
    //    Bounded; idempotent per (lens, site) via the deterministic incident key.
    let mut keys = Vec::new();
    for f in findings.iter().take(MAX_CARDS_PER_ROUND) {
        let incident = incident_key(f);
        let summary = format!(
            "loophole[{}]: {} ({}:{})",
            f.lens.slug(),
            f.hint,
            f.file,
            f.line
        );
        let body = card_body(f);
        let code = capture_incident(project, &incident, &summary, &body, "HEAD~1");
        if code == 0 {
            if !keys.contains(&incident) {
                keys.push(incident);
            }
        } else {
            print_or_exit(&format!(
                "[loophole] WARN: card write returned {code} for {incident}"
            ))?;
        }
    }
    let total = findings.len();
    if total > MAX_CARDS_PER_ROUND {
        // No silent cap: name what was dropped this round.
        print_or_exit(&format!(
            "[loophole] NOTE: {total} findings, capped at {MAX_CARDS_PER_ROUND}; rerun next round for the rest"
        ))?;
    }
    print_or_exit(&format!(
        "[loophole] round {round}: {} loophole card(s) recorded",
        keys.len()
    ))?;
    Ok(RoundOutcome {
        keys,
        total_findings: total,
    })
}

/// `kavach loophole loop` entry — the loop-until-dry convergence engine. Re-runs
/// sweep rounds, accumulating every (lens,site) incident key ever seen, until
/// `dry_rounds` CONSECUTIVE rounds surface no NEW key (convergence) OR `max_rounds`
/// is reached (the runaway brake). Each round still emits its own /tmp YAML and
/// records cards via the single-writer path; Kavach only DETECTS + RECORDS — the
/// subscription native agent fixes between rounds, shrinking the finding set until
/// it goes dry. Exit 0 on clean convergence; 1 if the cap was hit while still hot.
pub(crate) fn run_loop(project: &str, run_id: &str, dry_rounds: u32, max_rounds: u32) -> i32 {
    match run_loop_inner(project, run_id, dry_rounds, max_rounds) {
        Ok(converged) => i32::from(!converged),
        Err(io) => into_exit_code(io),
    }
}

fn run_loop_inner(
    project: &str,
    run_id: &str,
    dry_rounds: u32,
    max_rounds: u32,
) -> Result<bool, IoExit> {
    // dry_rounds == 0 is meaningless (would "converge" before any round); floor at 1.
    let need_dry = dry_rounds.max(1);
    // max_rounds < need_dry makes convergence structurally impossible (you cannot
    // get N consecutive dry rounds in fewer than N total rounds) — the loop would
    // run to the cap and report failure on a genuinely clean codebase. Raise the
    // cap to at least need_dry so a clean repo can actually converge. (boundary)
    let cap = max_rounds.max(need_dry);
    print_or_exit(&format!(
        "[loophole] loop start: run={run_id} need {need_dry} consecutive dry round(s), cap {cap}"
    ))?;

    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut consecutive_dry = 0_u32;
    let mut round = 0_u32;
    while round < cap {
        round = round.saturating_add(1);
        let outcome = sweep_round(project, run_id, round)?;

        // A round is dry when it added NO key absent from the accumulated set —
        // either zero findings, or every finding is a known (already-recorded) one.
        let fresh = outcome.keys.iter().filter(|k| !seen.contains(*k)).count();
        for k in outcome.keys {
            seen.insert(k);
        }
        consecutive_dry = next_streak(consecutive_dry, fresh);
        if fresh == 0 {
            print_or_exit(&format!(
                "[loophole] round {round} dry ({consecutive_dry}/{need_dry}); {} known site(s), {} raw finding(s)",
                seen.len(),
                outcome.total_findings
            ))?;
        } else {
            print_or_exit(&format!(
                "[loophole] round {round}: {fresh} NEW loophole site(s); streak reset"
            ))?;
        }
        if consecutive_dry >= need_dry {
            print_or_exit(&format!(
                "[loophole] CONVERGED after {round} round(s): {need_dry} consecutive dry, {} total site(s) seen",
                seen.len()
            ))?;
            return Ok(true);
        }
    }

    // Cap hit before convergence — name it; never claim a clean stop we didn't reach.
    print_or_exit(&format!(
        "[loophole] cap {cap} reached without {need_dry} consecutive dry round(s); {} site(s) seen, {consecutive_dry} dry in a row",
        seen.len()
    ))?;
    Ok(false)
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
        f.lens.slug(),
        f.file,
        f.line,
        f.hint,
        f.lens.slug()
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

/// True for a non-test Rust source. Test code (`tests.rs`, `*_test.rs`,
/// `*_tests.rs` sibling modules, anything under a `tests/` dir) is excluded — the
/// workspace lint contract already tolerates `unwrap`/`expect` there, so flagging
/// it floods the board with non-defects (the noise loophole that surfaced on the
/// first real sweep; this codebase's `_tests.rs` sibling convention is a second
/// variant of the same trap — pattern.code-scanner-must-exclude-test-code).
fn is_scannable_rust(path: &str) -> bool {
    let p = std::path::Path::new(path);
    if p.extension().is_none_or(|e| !e.eq_ignore_ascii_case("rs")) {
        return false;
    }
    let s = path.replace('\\', "/");
    if s.contains("/tests/") || s.starts_with("tests/") {
        return false;
    }
    p.file_name().and_then(|n| n.to_str()).is_some_and(|name| {
        // Exclude every test-file convention: tests.rs, and any stem carrying a
        // `_test` segment — `foo_test.rs`, `foo_tests.rs`, and sibling test-support
        // modules like `foo_test_menu.rs` / `foo_test_helpers.rs` (included via
        // `#[path] mod` under #[cfg(test)], so they're test code with no in-file
        // marker — the stop_signals_test_menu FP class).
        name != "tests.rs" && !name.trim_end_matches(".rs").contains("_test")
    })
}

/// Advance the consecutive-dry streak from one round's fresh-finding count. Zero
/// fresh sites extends the streak (saturating, no overflow); any fresh site
/// resets it to 0. Pure — the loop-until-dry convergence decision in isolation.
const fn next_streak(consecutive_dry: u32, fresh: usize) -> u32 {
    if fresh == 0 {
        consecutive_dry.saturating_add(1)
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_round_extends_streak() {
        assert_eq!(next_streak(0, 0), 1);
        assert_eq!(next_streak(1, 0), 2);
        assert_eq!(next_streak(4, 0), 5);
    }

    #[test]
    fn fresh_finding_resets_streak() {
        assert_eq!(next_streak(3, 1), 0, "any new site resets the streak");
        assert_eq!(next_streak(9, 7), 0);
    }

    #[test]
    fn streak_saturates_not_overflows() {
        assert_eq!(
            next_streak(u32::MAX, 0),
            u32::MAX,
            "no wraparound at the cap"
        );
    }

    #[test]
    fn excludes_every_test_file_naming_variant() {
        // Production sources are scanned.
        assert!(is_scannable_rust("crates/kavach-cli/src/cmd/loophole.rs"));
        // Every test convention is excluded — including the `_tests.rs` sibling
        // variant this codebase uses (the noise loophole's second face).
        assert!(!is_scannable_rust(
            "crates/kavach-chain/src/gates/aegis_tests.rs"
        ));
        assert!(!is_scannable_rust("crates/x/src/foo_test.rs"));
        assert!(!is_scannable_rust("crates/x/src/tests.rs"));
        assert!(!is_scannable_rust("crates/x/tests/integration.rs"));
        // Sibling test-support modules (test code with no in-file #[cfg(test)]).
        assert!(!is_scannable_rust(
            "crates/kavach-chain/src/stop_signals_test_menu.rs"
        ));
        assert!(!is_scannable_rust("crates/x/src/foo_test_helpers.rs"));
        // A production file that merely contains "test" elsewhere stays scannable.
        assert!(is_scannable_rust("crates/x/src/latest.rs"));
        assert!(is_scannable_rust("crates/x/src/contest.rs"));
        // Non-Rust never scanned.
        assert!(!is_scannable_rust("crates/x/src/foo.toml"));
    }

    #[test]
    fn convergence_takes_need_dry_consecutive_rounds() {
        // Simulate the loop's streak accounting: a fresh round mid-run resets it,
        // so convergence needs `need_dry` clean rounds in a row, not cumulative.
        let need_dry = 2_u32;
        let rounds_fresh = [1_usize, 0, 1, 0, 0]; // dirty, dry, dirty, dry, dry
        let mut streak = 0_u32;
        let mut converged_at = None;
        for (i, &fresh) in rounds_fresh.iter().enumerate() {
            streak = next_streak(streak, fresh);
            if streak >= need_dry {
                converged_at = Some(i.saturating_add(1));
                break;
            }
        }
        assert_eq!(
            converged_at,
            Some(5),
            "two consecutive dry rounds end at round 5"
        );
    }
}
