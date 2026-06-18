//! Evidence-bound status-promotion gate (owner directive 2026-06-18).
//!
//! ROOT CAUSE THIS GATE FIXES: a roadmap card reached `done`/`verified` as a bare
//! DB state write on the agent's say-so.
//!
//! No artifact/build/test proof was bound to the write. The only witness run lived
//! later in the auto-verify path and a direct `status-update`/`verify_card`
//! bypassed it. Result: the DB durably stored CLAIMS as if they were evidence
//! (false "done"). This gate binds the proof to the claim at the agent-facing
//! entry point: a promotion to `done` or `verified` MUST be backed by a fresh
//! workspace witness pass, else it is refused.
//!
//! `§evidence_over_inference` + `§three_witness_verify`, enforced in code (not prose).

use crate::gates::stop_dispatch::verify::witness::{
    WitnessRun, run_workspace_witnesses, witness_root_from_card,
};

/// Whether a requested status promotion is allowed, given the objective witnesses.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusGateVerdict {
    /// Not a promotion to `done`/`verified` (e.g. `todo`/`in_progress`) — no proof
    /// required; the gate does not apply.
    NotGated,
    /// Witnesses passed (build+test+diff) — the promotion is evidence-backed.
    Allowed,
    /// Witnesses FAILED or could not spawn — refuse the promotion (false claim).
    RefusedWitnessFailed,
    /// Work is unprovable here (non-Rust + no `KAVACH_VERIFY_CMD`). Refuse a
    /// `verified` promotion fail-closed; the agent must supply a verify command.
    RefusedUnprovable,
}

/// True iff `status` is a completion-claim status that must be evidence-backed.
/// Parsed through the typed `MemoryStatus` boundary; the complete-set lives on
/// the enum (`is_complete`), and a non-canonical value is not a completion claim.
#[must_use]
fn is_completion_status(status: &str) -> bool {
    status
        .parse::<kavach_types::MemoryStatus>()
        .is_ok_and(kavach_types::MemoryStatus::is_complete)
}

/// Gate a roadmap status promotion on the objective workspace witnesses.
///
/// Only `roadmap` completion statuses (`done`/`verified`) are gated; everything
/// else returns [`StatusGateVerdict::NotGated`]. The witnesses run in the agent's
/// CWD (the project workspace), exactly as the auto-verify path does. This is the
/// EVIDENCE binding the prior design lacked: a missing/failing build can no longer
/// be promoted to a completion status on self-report alone.
/// `card_content` is the promoting card's body, scanned for a per-card
/// `WITNESS_ROOT:` hint so a cross-repo card is verified in the repo its code
/// actually lives in (not the dispatch CWD). Pass `""` when no content is at hand.
#[must_use]
pub fn verify_status_promotion(
    category: &str,
    status: &str,
    card_content: &str,
) -> StatusGateVerdict {
    if category != "roadmap" || !is_completion_status(status) {
        return StatusGateVerdict::NotGated;
    }
    let card_root = witness_root_from_card(card_content);
    match run_workspace_witnesses(card_root.as_deref()) {
        WitnessRun::Passed => StatusGateVerdict::Allowed,
        WitnessRun::Failed | WitnessRun::SpawnError => StatusGateVerdict::RefusedWitnessFailed,
        WitnessRun::Unprovable => StatusGateVerdict::RefusedUnprovable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_roadmap_category_is_not_gated() {
        assert_eq!(
            verify_status_promotion("decision", "done", ""),
            StatusGateVerdict::NotGated
        );
    }

    #[test]
    fn todo_status_is_not_gated() {
        assert_eq!(
            verify_status_promotion("roadmap", "todo", ""),
            StatusGateVerdict::NotGated
        );
    }

    #[test]
    fn in_progress_status_is_not_gated() {
        assert_eq!(
            verify_status_promotion("roadmap", "in_progress", ""),
            StatusGateVerdict::NotGated
        );
    }

    #[test]
    fn done_is_a_completion_status() {
        assert!(is_completion_status("done"));
    }

    #[test]
    fn verified_is_a_completion_status() {
        assert!(is_completion_status("verified"));
    }

    #[test]
    fn todo_is_not_a_completion_status() {
        assert!(!is_completion_status("todo"));
    }
}
