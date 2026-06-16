//! H4 fail-closed auto-merge gate (CLI shell). Gathers the PR's CI status and
//! changed-file list via `gh`, reads the master switch from the environment, and
//! delegates the ALLOW/DENY verdict to the pure `decide` module. Kavach DECIDES;
//! it never performs the merge — exit 0 (allow) lets the caller merge, non-zero
//! (deny) blocks it. Any gather failure DENIES (fail-closed).
//! SOURCE: roadmap heal.unit.auto-merge-gate · decision.heal.self-healing-pipeline-architecture.

mod decide;

use crate::cmd::io_safe::{into_exit_code, print_or_exit};
use decide::decide;
use std::process::Command;

/// Master-switch env var. Unset / not exactly "1" ⇒ auto-merge OFF (default).
const SWITCH_ENV: &str = "KAVACH_HEAL_AUTOMERGE";

/// `kavach heal merge-gate` entry. Exit 0 = allow auto-merge, non-zero = deny.
pub(crate) fn run(pr: u64, witness_pass: bool) -> i32 {
    let enabled = std::env::var(SWITCH_ENV).is_ok_and(|v| v == "1");
    let ci_green = ci_is_green(pr);
    let changed = changed_files(pr);
    let d = decide(enabled, ci_green, witness_pass, &changed);

    if d.allow {
        return match print_or_exit(&format!("[heal merge-gate] ALLOW: PR #{pr} clears all gates")) {
            Ok(()) => 0,
            Err(io) => into_exit_code(io),
        };
    }
    // Deny: print every failing reason so the operator sees the full picture.
    if let Err(io) = print_or_exit(&format!("[heal merge-gate] DENY: PR #{pr}")) {
        return into_exit_code(io);
    }
    for r in &d.reasons {
        if let Err(io) = print_or_exit(&format!("  - {r}")) {
            return into_exit_code(io);
        }
    }
    1
}

/// True ONLY if `gh pr checks <pr>` reports every required check passing.
/// ANY failure to determine status (gh missing, network, non-zero exit) ⇒ false
/// (fail-closed: we never auto-merge on an unknown CI state).
fn ci_is_green(pr: u64) -> bool {
    Command::new("gh")
        .args(["pr", "checks", &pr.to_string(), "--required"])
        .output()
        .is_ok_and(|o| o.status.success())
}

/// The PR's changed files via `gh pr diff --name-only`. Empty on any error —
/// the decider treats an empty diff as "cannot prove safety" and DENIES, so a
/// gather failure can never accidentally allow a merge.
fn changed_files(pr: u64) -> Vec<String> {
    let Ok(out) = Command::new("gh")
        .args(["pr", "diff", &pr.to_string(), "--name-only"])
        .output()
    else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::to_owned)
        .collect()
}
