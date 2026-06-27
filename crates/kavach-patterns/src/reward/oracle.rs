// split: multidimensional ground-truth oracle — the RLVR verifier ensemble that
// scores a completion claim against orthogonal artifacts the agent cannot fake.
// SOURCE: decision.harness-reward-ground-truth-oracle (multidimensional rewrite).
//
// WHY an ensemble, not one marker list: a single hardcoded signal IS the loophole
// — one runner phrasing failure differently slips through. Per the RLVR literature
// (Weaver, Stanford 2026; SWE-Marathon; arXiv:2509.15557) a weighted vote across
// INDEPENDENT dimensions is hard to game: an agent must defeat every orthogonal
// channel at once, and a miscalibrated channel can ABSTAIN instead of forcing a
// wrong vote (Youden-J<0 collapse guard). No single dimension is decisive.
//
// Config (weights / margin / penalty / failure vocab) is DATA, not source literals
// — `OracleConfig` carries a hardcoded `Default` fail-safe, and the engine layer
// overrides it from a research-refreshed kavach-DB row. That is what removes the
// "hardcoded parameters" objection at its root.
use crate::eval_replay::{EventKind, EventOutcome, TrajectoryEvent};
/// One dimension's independent verdict on a completion claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DimVerdict {
    /// This dimension's artifact agrees the claim is true.
    Agree,
    /// This dimension's artifact contradicts the claim.
    Contradict,
    /// This dimension has no signal here — it MUST abstain rather than guess, so a
    /// blind dimension can never poison the vote (the J<0-collapse guard).
    Abstain,
}
/// Tunable oracle parameters.
///
/// The hardcoded [`Default`] is the fail-safe served when the DB override is
/// absent/unreachable; the engine layer replaces it with a research-refreshed row.
/// Nothing here is a source-literal the vote structurally depends on.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct OracleConfig {
    /// Weight of the process-exit dimension (host error/interrupt flag).
    pub w_exit: i64,
    /// Weight of the output-failure-vocab dimension.
    pub w_output: i64,
    /// Contradiction wins only when `contradict` ≥ `agree` + this margin, so a lone
    /// weak dimension cannot flip the verdict — quorum, not dictator.
    pub margin: i64,
    /// The dominant penalty applied on a proven contradiction. Must outweigh the
    /// `FILE_LANDED` floor plus any plausible credit stack in one turn.
    pub penalty: i64,
    /// Failure vocabulary for the output dimension — DATA, refreshable from the DB,
    /// never a hardcoded slice the verdict structurally depends on.
    pub failure_vocab: Vec<String>,
}
impl Default for OracleConfig {
    fn default() -> Self {
        Self {
            w_exit: 2,
            w_output: 2,
            margin: 1,
            penalty: -100,
            failure_vocab: DEFAULT_FAILURE_VOCAB
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
        }
    }
}
/// Compiled fail-safe failure vocabulary. Only the seed for `OracleConfig::default`
/// — the live system serves the DB-refreshed list; this is what ships when the DB
/// is unreachable so the oracle is never worse than its compiled floor.
const DEFAULT_FAILURE_VOCAB: &[&str] = &[
    "error[e",
    "could not compile",
    "test result: failed",
    "panicked at",
    "compilation failed",
    "build failed",
    "error: test failed",
    "failures:",
];
/// Completion/success vocabulary for the claim side. Broad on purpose: a penalty
/// only lands when the dimension vote ALSO contradicts, so over-matching is benign.
const DONE_MARKERS: &[&str] = &[
    "done",
    "complete",
    "completed",
    "finished",
    "fixed",
    "passing",
    "passes",
    "green",
    "shipped",
    "landed",
    "works",
    "working",
    "resolved",
    "success",
    "all set",
    "ready",
];
/// `true` iff a Stop in the trajectory narrates completion — the claim under test.
fn claims_completion(events: &[TrajectoryEvent]) -> bool {
    events.iter().any(|e| match &e.event_kind {
        EventKind::Stop { final_message } => {
            let m = final_message.to_ascii_lowercase();
            DONE_MARKERS.iter().any(|k| m.contains(k))
        }
        _ => false,
    })
}
// ── Orthogonal dimensions ───────────────────────────────────────────────────
// Each reads a DIFFERENT artifact, so gaming one does not blind the others.
/// Process-exit dimension: the recorded objective `EventOutcome` on verify events.
/// Any `Failure` → Contradict; some `Success` and no `Failure` → Agree; no recorded
/// outcome at all → Abstain (the host never flagged anything — no signal to vote).
fn dim_process_exit(events: &[TrajectoryEvent]) -> DimVerdict {
    let mut saw_success = false;
    for ev in events {
        match ev.outcome {
            Some(EventOutcome::Failure) => return DimVerdict::Contradict,
            Some(EventOutcome::Success) => saw_success = true,
            None => {}
        }
    }
    if saw_success {
        DimVerdict::Agree
    } else {
        DimVerdict::Abstain
    }
}
/// Output-failure-vocab dimension: a Bash command's own text contains a failure
/// token from the (DB-refreshable) vocab. This is the dimension that USED to be the
/// whole oracle — now it is one vote of several, and its vocab is data. It is
/// asymmetric on purpose: a failure token is hard evidence (`Contradict`), but the
/// ABSENCE of one is NOT proof of success (`Abstain`) — only the process-exit dim,
/// reading a different artifact, can Agree. Keeping the two dims non-redundant is
/// the orthogonality the ensemble depends on.
fn dim_output_failure(events: &[TrajectoryEvent], vocab: &[String]) -> DimVerdict {
    for ev in events {
        if let EventKind::Bash { command } = &ev.event_kind {
            let lower = command.to_ascii_lowercase();
            if vocab.iter().any(|t| lower.contains(t.as_str())) {
                return DimVerdict::Contradict;
            }
        }
    }
    DimVerdict::Abstain
}
/// The weighted contradiction tally across all dimensions for `events` under
/// `cfg`. Returns `(contradict_score, agree_score)`; abstaining dimensions add to
/// neither, so a blind channel is inert.
fn tally(events: &[TrajectoryEvent], cfg: &OracleConfig) -> (i64, i64) {
    let dims: [(DimVerdict, i64); 2] = [
        (dim_process_exit(events), cfg.w_exit),
        (dim_output_failure(events, &cfg.failure_vocab), cfg.w_output),
    ];
    let mut contradict = 0_i64;
    let mut agree = 0_i64;
    for (verdict, weight) in dims {
        match verdict {
            DimVerdict::Contradict => contradict = contradict.saturating_add(weight),
            DimVerdict::Agree => agree = agree.saturating_add(weight),
            DimVerdict::Abstain => {}
        }
    }
    (contradict, agree)
}
/// Oracle penalty under an explicit [`OracleConfig`] — the multidimensional vote.
///
/// `0` unless (a) a Stop claims completion AND (b) the weighted dimension vote
/// reaches quorum: `contradict ≥ agree + margin`. No single dimension is decisive,
/// abstentions are inert, and the failure vocabulary is data — so neither a brittle
/// marker list nor a magic constant is load-bearing.
#[must_use]
pub fn oracle_penalty_with(events: &[TrajectoryEvent], cfg: &OracleConfig) -> i64 {
    if !claims_completion(events) {
        return 0;
    }
    let (contradict, agree) = tally(events, cfg);
    if contradict > 0 && contradict >= agree.saturating_add(cfg.margin) {
        cfg.penalty
    } else {
        0
    }
}
/// Oracle penalty under the compiled fail-safe [`OracleConfig::default`]. The
/// engine layer prefers [`oracle_penalty_with`] threaded with a DB-refreshed config.
#[must_use]
pub fn oracle_penalty(events: &[TrajectoryEvent]) -> i64 {
    oracle_penalty_with(events, &OracleConfig::default())
}
#[cfg(test)]
#[path = "oracle_test.rs"]
mod tests;
