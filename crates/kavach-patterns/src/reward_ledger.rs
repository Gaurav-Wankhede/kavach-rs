//! Balance-sheet ledger over a scored trajectory.
//!
//! Operator directive 2026-06-17: "its Ledger to the Kavach Database as Records of
//! Balance Sheet with Mistakes and Learning Awareness". PURE: decomposes a
//! trajectory's events into typed CREDIT (verified work) and DEBIT (penalties:
//! gate-block, deferral-handoff) line items + a net. The engine persists the
//! rendered row at Stop; this module never does I/O so it is deterministic +
//! unit-testable. See `reward.rs` for the scalar `score_trajectory`; this is its
//! itemized double-entry view.
use crate::eval_replay::{EventKind, ReplaySeverity, TrajectoryEvent, replay_event};
use crate::reward::score_trajectory;
/// One double-entry line. `weight` is signed: credits positive, debits negative.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LedgerLine {
    /// Stable kind tag (`build`/`test`/`file` credit; `gate_block`/`deferral_handoff` debit).
    pub kind: &'static str,
    /// Signed point weight this line contributes to the net.
    pub weight: i64,
}
/// A turn's balance sheet: itemized credits + debits and the net (== `score_trajectory`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TurnLedger {
    /// Verified-work line items (positive weight).
    pub credits: Vec<LedgerLine>,
    /// Penalty line items (negative weight) — the "mistakes" half of the sheet.
    pub debits: Vec<LedgerLine>,
    /// Net = sum(credits) + sum(debits). Equals the scalar reward for the turn.
    pub net: i64,
}
fn credit_for(event: &TrajectoryEvent) -> Option<LedgerLine> {
    match &event.event_kind {
        EventKind::Bash { command } if crate::reward::is_real_verify(command) => Some(LedgerLine {
            kind: if command.contains("test") || command.contains("nextest") {
                "test"
            } else {
                "build"
            },
            weight: score_trajectory(std::slice::from_ref(event)).max(0),
        }),
        EventKind::Write { .. } => {
            let w = score_trajectory(std::slice::from_ref(event)).max(0);
            (w > 0).then_some(LedgerLine {
                kind: "file",
                weight: w,
            })
        }
        _ => None,
    }
}
fn debit_for(event: &TrajectoryEvent) -> Option<LedgerLine> {
    // A debit is any negative contribution: a gate Block, or a deferral-handoff Stop.
    let w = score_trajectory(std::slice::from_ref(event));
    if w >= 0 {
        return None;
    }
    let kind = match &event.event_kind {
        EventKind::Stop { .. } => "deferral_handoff",
        _ if replay_event(event)
            .iter()
            .any(|o| o.severity == ReplaySeverity::Block) =>
        {
            "gate_block"
        }
        _ => "penalty",
    };
    Some(LedgerLine { kind, weight: w })
}
/// Build the itemized balance sheet for a turn. `net` is guaranteed to equal
/// `score_trajectory(events)` so the ledger can never disagree with the scalar.
#[must_use]
pub fn build_ledger(events: &[TrajectoryEvent]) -> TurnLedger {
    let credits: Vec<_> = events.iter().filter_map(credit_for).collect();
    let debits: Vec<_> = events.iter().filter_map(debit_for).collect();
    TurnLedger {
        credits,
        debits,
        net: score_trajectory(events),
    }
}
#[cfg(test)]
#[path = "reward_ledger_test.rs"]
mod tests;
