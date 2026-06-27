// split: GEPA reflective mutator (P2). Turns a low-scoring trajectory + its gate
// fires + RCA into ONE structured, PROPOSED gate-rule edit. The "text gradient".
// WHY single-edit + proposed-not-applied + the LLM-call-as-boundary design:
// kavach-db decision.harness-reflect-mutator / roadmap.unit.harness-reflect-mutator.
//
// INV-5: a MutationProposal is DATA — it is NEVER auto-applied to a live gate.
// AC-2: one reflection call per failed trajectory yields at most one proposal.

use crate::eval_replay::{GateOutcome, TrajectoryEvent, replay_trajectory};
use std::fmt::Write as _;

/// The kind of rule edit a reflection can propose. Closed set — a proposal can
/// only ever be one of these, so an "illegal" edit cannot be represented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EditKind {
    /// The gate over-fired (false positive) — propose loosening / adding an exception.
    Loosen,
    /// The gate under-fired (missed a real defect) — propose tightening.
    Tighten,
}

impl EditKind {
    fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "loosen" => Some(Self::Loosen),
            "tighten" => Some(Self::Tighten),
            _ => None,
        }
    }
}

/// Exactly ONE proposed gate-rule edit (AC-2). Constructed only via `parse_proposal`,
/// so every field is non-empty and the gate name is one the harness actually has.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MutationProposal {
    /// The gate this edit targets (must match a known gate name).
    pub gate: String,
    /// Loosen (over-fire) or tighten (under-fire).
    pub edit: EditKind,
    /// One-line, human-reviewable rationale grounded in the trajectory.
    pub rationale: String,
    /// The session id whose trajectory motivated this proposal (provenance).
    pub from_session: String,
}

/// The single-LLM-call boundary. Real impl wraps the model API (wired at P4);
/// tests inject a deterministic stub. Returns `None` when the model declines or
/// errors — the caller fails closed (no proposal).
pub trait Reflector {
    /// Run one reflection over `prompt`. `None` = no usable response.
    fn reflect(&self, prompt: &str) -> Option<String>;
}

/// The gate names a proposal is allowed to target. A proposal naming anything
/// outside this set is rejected (fail-closed) — prevents a hallucinated gate.
const KNOWN_GATES: &[&str] = &[
    "destructive_cli_guard",
    "solid_guard",
    "dsa_guard",
    "database_ops_guard",
    "pii_data_guard",
    "migration_safety_guard",
    "webhook_signature_guard",
    "observability_guard",
    "finops_guard",
    "axum_guard",
    "false_completion_detector",
];

/// Build the reflection prompt from a scored-low trajectory.
///
/// Pure + deterministic (same inputs → same prompt): the events, the gates that
/// fired with their reasons, and any operator RCA. The model is asked to return
/// exactly one edit in a fixed `gate|edit|rationale` line format.
#[must_use]
pub fn assemble_reflection_prompt(
    events: &[TrajectoryEvent],
    rca: &str,
    session_id: &str,
) -> String {
    let mut p = String::with_capacity(512);
    p.push_str(
        "You are refining a code-review gate harness. Below is one agent session that \
         scored poorly. Propose EXACTLY ONE gate-rule edit that would have improved it.\n\
         Respond with ONE line: <gate>|<loosen|tighten>|<one-line rationale>\n\
         Use loosen if a gate over-fired (false positive), tighten if it missed a defect.\n\n",
    );
    writeln!(p, "session: {session_id}").ok();
    if !rca.trim().is_empty() {
        writeln!(p, "operator_rca: {}", rca.trim()).ok();
    }
    p.push_str("\ngate_fires:\n");
    let mut any = false;
    for (idx, outcomes) in replay_trajectory(events) {
        for GateOutcome { gate, message, .. } in outcomes {
            any = true;
            writeln!(p, "  step {idx}: {gate} — {message}").ok();
        }
    }
    if !any {
        p.push_str("  (no gate fired)\n");
    }
    p.push_str("\nallowed_gates: ");
    p.push_str(&KNOWN_GATES.join(", "));
    p.push('\n');
    p
}

/// Parse a model response into a validated `MutationProposal`.
///
/// Fail-closed: returns `None` for empty input, the wrong shape, an unknown gate,
/// or a blank rationale — a malformed reflection MUST NOT yield a garbage proposal.
#[must_use]
pub fn parse_proposal(response: &str, from_session: &str) -> Option<MutationProposal> {
    // Take the first valid line; SKIP (not abort on) any model preamble or noise.
    response.lines().find_map(|line| {
        let mut parts = line.trim().splitn(3, '|');
        let gate = parts.next()?.trim();
        let edit = EditKind::parse(parts.next()?)?;
        let rationale = parts.next()?.trim();
        (!rationale.is_empty() && KNOWN_GATES.contains(&gate)).then(|| MutationProposal {
            gate: gate.to_owned(),
            edit,
            rationale: rationale.to_owned(),
            from_session: from_session.to_owned(),
        })
    })
}

/// One reflection over one failed trajectory (AC-2).
///
/// Assemble the prompt, run the injected reflector, parse the result. `None` if the
/// model declines or returns an unusable proposal. INV-5: the proposal is DATA, not
/// an applied edit.
#[must_use]
pub fn reflect_once<R: Reflector>(
    reflector: &R,
    events: &[TrajectoryEvent],
    rca: &str,
    session_id: &str,
) -> Option<MutationProposal> {
    let prompt = assemble_reflection_prompt(events, rca, session_id);
    let response = reflector.reflect(&prompt)?;
    parse_proposal(&response, session_id)
}

#[cfg(test)]
#[path = "reflect_test.rs"]
#[cfg(test)]
#[path = "reflect_test.rs"]
mod tests;