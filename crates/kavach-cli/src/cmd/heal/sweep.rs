//! H3 proactive bug-hunt: run the repo's NON-AI quality gates and, for each
//! gate that FAILS, capture a self-heal card so the autonomous loop fixes the
//! defect before CI ever sees it. Detection only — Kavach never calls an LLM;
//! the subscription native agent does the fix.
//! SOURCE: decision.heal.self-healing-pipeline-architecture · roadmap heal.unit.proactive-bughunt.

use super::capture_incident;
use crate::cmd::io_safe::{IoExit, into_exit_code, print_or_exit};
use std::process::Command;

/// One non-AI gate: a deterministic detector whose non-zero exit is a defect.
/// `incident` is the STABLE card-key suffix → re-sweeping the same failing gate
/// UPDATES one card (idempotent; replay loophole closed at `capture::card_key`).
struct Gate {
    /// Stable incident id suffix, e.g. `sweep-clippy`.
    incident: &'static str,
    /// Human summary for the card title.
    summary: &'static str,
    /// Program + args to run (the gate command).
    argv: &'static [&'static str],
}

/// The proactive gate set — the cheap, deterministic detectors that catch the
/// defect classes CI would otherwise reject. Ordered cheap→expensive.
const GATES: &[Gate] = &[
    Gate {
        incident: "sweep-check",
        summary: "proactive sweep: cargo check --workspace failed",
        argv: &["cargo", "check", "--workspace", "--all-targets"],
    },
    Gate {
        incident: "sweep-clippy",
        summary: "proactive sweep: clippy -D warnings failed",
        argv: &[
            "cargo",
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    },
    Gate {
        incident: "sweep-machete",
        summary: "proactive sweep: cargo machete found unused dependencies",
        argv: &["cargo", "machete"],
    },
];

/// `kavach heal sweep` entry. Runs every gate; for each failure, captures a card.
/// Returns 0 even when gates fail — a found defect is a SUCCESSFUL sweep that
/// enqueued work (exit non-zero would falsely read as "the sweep itself broke").
/// A gate binary that cannot spawn (e.g. `cargo machete` not installed) is
/// skipped with a notice, never a captured card (no false-positive defect).
pub(crate) fn run(project: &str) -> i32 {
    match run_inner(project) {
        Ok(()) => 0,
        Err(io) => into_exit_code(io),
    }
}

/// Sweep body. Any progress-line IO failure propagates as `Err` (a broken stdout
/// aborts the sweep rather than silently dropping the line — the must-use Result
/// is handled, never discarded).
fn run_inner(project: &str) -> Result<(), IoExit> {
    let mut captured = 0_u32;
    let mut skipped = 0_u32;
    for gate in GATES {
        match run_gate(gate) {
            GateOutcome::Pass => {}
            GateOutcome::Fail(output) => {
                // diff_base HEAD~1: a sweep runs on the working tree, so the most
                // useful "what changed" is the last commit's delta.
                let code =
                    capture_incident(project, gate.incident, gate.summary, &output, "HEAD~1");
                if code == 0 {
                    captured = captured.saturating_add(1);
                } else {
                    // The capture write failed (daemon/db) — surface, don't swallow.
                    print_or_exit(&format!(
                        "[heal sweep] WARN: gate '{}' failed but card write returned {code}",
                        gate.incident
                    ))?;
                }
            }
            GateOutcome::Unspawnable(why) => {
                skipped = skipped.saturating_add(1);
                print_or_exit(&format!("[heal sweep] skip '{}': {why}", gate.incident))?;
            }
        }
    }
    print_or_exit(&format!(
        "[heal sweep] done: {captured} card(s) captured, {skipped} gate(s) skipped"
    ))
}

/// Result of running one gate.
enum GateOutcome {
    /// Gate exited 0 — no defect.
    Pass,
    /// Gate exited non-zero — the combined stdout+stderr is the failure context.
    Fail(String),
    /// The gate command could not be spawned (missing tool) — skip, not a defect.
    Unspawnable(String),
}

/// Run one gate command, returning its outcome. Combines stdout+stderr so the
/// captured card carries the full compiler/linter diagnostic.
fn run_gate(gate: &Gate) -> GateOutcome {
    let Some((prog, args)) = gate.argv.split_first() else {
        return GateOutcome::Unspawnable("empty gate argv".to_owned());
    };
    let out = match Command::new(prog).args(args).output() {
        Ok(o) => o,
        Err(e) => return GateOutcome::Unspawnable(format!("cannot spawn {prog}: {e}")),
    };
    if out.status.success() {
        return GateOutcome::Pass;
    }
    let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&out.stderr));
    GateOutcome::Fail(combined)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_incident_ids_are_unique_and_sweep_prefixed() {
        // Unique ids → one card per gate (no two gates collide on a card key).
        let mut ids: Vec<&str> = GATES.iter().map(|g| g.incident).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len(), "gate incident ids must be unique");
        assert!(
            GATES.iter().all(|g| g.incident.starts_with("sweep-")),
            "sweep cards are namespaced under 'sweep-'"
        );
    }

    #[test]
    fn unspawnable_gate_is_skipped_not_failed() {
        let gate = Gate {
            incident: "sweep-nonexistent",
            summary: "x",
            argv: &["definitely-not-a-real-binary-xyz"],
        };
        assert!(
            matches!(run_gate(&gate), GateOutcome::Unspawnable(_)),
            "a missing tool must skip, never capture a false-positive defect"
        );
    }

    #[test]
    fn passing_gate_is_pass() {
        let gate = Gate {
            incident: "sweep-true",
            summary: "x",
            argv: &["true"],
        };
        assert!(matches!(run_gate(&gate), GateOutcome::Pass));
    }

    #[test]
    fn failing_gate_captures_output() {
        // `false` exits 1 with no output → Fail with empty-but-present context.
        let gate = Gate {
            incident: "sweep-false",
            summary: "x",
            argv: &["false"],
        };
        assert!(matches!(run_gate(&gate), GateOutcome::Fail(_)));
    }
}
