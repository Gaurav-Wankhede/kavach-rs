// split: Project-adaptive reward scorer over an eval_replay trajectory (INV-1 of
// the GEPA harness self-optimization plan). A RewardRubric (weighted signal
// vector) drives scoring so every stack — not just Rust/cargo — scores correctly.
// WHY this design: decision.arch.harness-rlaif-project-adaptive-rubric.
// WHY un-gameable weighting + reward-hack RCA: decision.harness-reward-ungameable.

pub mod oracle;
pub mod presets;
pub mod rubric;
pub mod semantic_deferral;

use crate::eval_replay::{EventKind, ReplaySeverity, TrajectoryEvent, replay_event};
use rubric::{EventClass, RewardRubric};

// Named weight constants — the canonical scalar weights the default Rust rubric
// uses. Re-exported for tests + the ledger so call sites name the weight rather
// than a magic number. The rubric presets hold the authoritative copies.
/// A landed `Write` (cheap alone).
pub const FILE_LANDED: i64 = 1;
/// An always-pass / no-op test — DELIBERATELY zero (reward-hack guard AC-5).
pub const VACUOUS_TEST: i64 = 0;
/// Deferral/false-blocker handoff penalty (stack-independent; dominates any single build).
pub const DEFERRAL_HANDOFF_PENALTY: i64 = presets::DEFERRAL_WEIGHT;

/// Text + class of an event, for rubric matching.
fn event_text(event: &TrajectoryEvent) -> (EventClass, &str) {
    match &event.event_kind {
        EventKind::Bash { command } => (EventClass::Bash, command),
        EventKind::Write { content, .. } => (EventClass::Write, content),
        EventKind::Stop { final_message } => (EventClass::Stop, final_message),
        EventKind::Tool { .. } => (EventClass::Bash, ""),
    }
}

/// The reward contribution of one event under `rubric`. Sums every matching
/// rule's weight; a `Write` whose body trips the rubric's vacuous-guard forfeits
/// its positive (credit) rules — the reward-hack guard (AC-5).
fn score_event(event: &TrajectoryEvent, rubric: &RewardRubric) -> i64 {
    // Gate Blocks are scored from replay severity (stack-independent), not a pattern.
    let gate_penalty = i64::try_from(
        replay_event(event)
            .iter()
            .filter(|o| o.severity == ReplaySeverity::Block)
            .count(),
    )
    .map_or(0, |n| n.saturating_mul(presets::GATE_BLOCK_WEIGHT));

    let (class, text) = event_text(event);
    let is_vacuous = class == EventClass::Write
        && rubric.vacuous_guard.as_ref().is_some_and(|g| g.is_match(text));

    let matched: Vec<_> = rubric
        .rules
        .iter()
        .filter(|r| r.applies_to == class && r.pattern.is_match(text))
        // A vacuous Write forfeits the TEST bonus (the reward-hack target) but
        // keeps the file-landed floor (weight 1) and never touches debits — an
        // always-pass test still "landed a file", it just earns no test credit.
        .filter(|r| !(is_vacuous && r.weight > FILE_LANDED))
        .collect();

    let signal = matched
        .iter()
        .map(|r| r.weight)
        .fold(0_i64, i64::saturating_add);

    // Semantic-deferral backstop (card semantic-deferral-detector): the literal
    // `deferral_pattern()` regex is the cheap first pass; when it did NOT fire on
    // a Stop, run the paraphrase-robust judge. A positive applies the SAME
    // `DEFERRAL_HANDOFF_PENALTY` once, so a reworded handoff cannot dodge the
    // debit. Gated on regex-miss → no double-penalty when the regex already hit.
    let regex_caught_deferral =
        matched.iter().any(|r| r.weight == DEFERRAL_HANDOFF_PENALTY);
    let semantic_penalty = if class == EventClass::Stop
        && !regex_caught_deferral
        && semantic_deferral::is_semantic_deferral(text)
    {
        DEFERRAL_HANDOFF_PENALTY
    } else {
        0
    };

    signal
        .saturating_add(gate_penalty)
        .saturating_add(semantic_penalty)
}

/// Deterministic reward of a whole trajectory under an explicit `rubric` (AC-1).
/// Read-only over `events` (INV-2): same input -> same output.
#[must_use]
pub fn score_trajectory_with(events: &[TrajectoryEvent], rubric: &RewardRubric) -> i64 {
    score_trajectory_full(events, rubric, &oracle::OracleConfig::default())
}

/// Deterministic reward under an explicit `rubric` AND an explicit oracle `cfg`.
///
/// The engine threads a DB-refreshed `cfg`; the back-compat [`score_trajectory_with`]
/// passes the compiled default. The oracle's weighted dimension vote contributes a
/// dominant negative on a proven contradiction, leaving the self-report intact on
/// agreement / insufficient evidence.
#[must_use]
pub fn score_trajectory_full(
    events: &[TrajectoryEvent],
    rubric: &RewardRubric,
    cfg: &oracle::OracleConfig,
) -> i64 {
    let self_report = events
        .iter()
        .map(|e| score_event(e, rubric))
        .fold(0_i64, i64::saturating_add);
    self_report.saturating_add(oracle::oracle_penalty_with(events, cfg))
}

/// Score under the default Rust/cargo rubric (back-compat: the original API).
/// The engine uses [`score_trajectory_with`] + the project's `gate.reward_rubric`.
#[must_use]
pub fn score_trajectory(events: &[TrajectoryEvent]) -> i64 {
    score_trajectory_with(events, &presets::rust_cargo())
}

/// `true` iff `command` is a real verify under the Rust default rubric — kept for
/// the ledger's credit-classification (a build/test line item).
#[must_use]
pub fn is_real_verify(command: &str) -> bool {
    presets::rust_cargo()
        .rules
        .iter()
        .any(|r| r.applies_to == EventClass::Bash && r.weight > 0 && r.pattern.is_match(command))
}

#[cfg(test)]
#[path = "reward_test.rs"]
mod tests;
