//! Tests for the GEPA reflective mutator (P2).
//!
//! AC-2: one reflection over a failed trajectory yields at most one proposal.
//! INV-5: a proposal is DATA — these tests never apply it to a gate.
//! Fail-closed: malformed / hallucinated model output yields NO proposal.
use super::*;
use crate::eval_replay::{EventKind, TrajectoryEvent};

/// A deterministic stand-in for the LLM call — returns a canned response.
struct StubReflector(&'static str);
impl Reflector for StubReflector {
    fn reflect(&self, _prompt: &str) -> Option<String> {
        Some(self.0.to_owned())
    }
}

/// A reflector that always declines (model error / empty).
struct SilentReflector;
impl Reflector for SilentReflector {
    fn reflect(&self, _prompt: &str) -> Option<String> {
        None
    }
}

fn bash(cmd: &str) -> TrajectoryEvent {
    TrajectoryEvent {
        timestamp_ms: 0,
        session_id: "t".into(),
        event_kind: EventKind::Bash {
            command: cmd.into(),
        },
        outcome: None,
    }
}

#[test]
fn prompt_is_deterministic() {
    let traj = vec![bash("rm -rf /")];
    let a = assemble_reflection_prompt(&traj, "over-fired on a safe cleanup", "s1");
    let b = assemble_reflection_prompt(&traj, "over-fired on a safe cleanup", "s1");
    assert_eq!(a, b);
}

#[test]
fn prompt_includes_gate_fires_and_rca() {
    // rm -rf / triggers destructive_cli_guard — the fire must surface in the prompt.
    let traj = vec![bash("rm -rf /")];
    let p = assemble_reflection_prompt(&traj, "the dir was a scratch temp", "s1");
    assert!(
        p.contains("destructive_cli_guard"),
        "gate fire must be in the prompt"
    );
    assert!(p.contains("operator_rca: the dir was a scratch temp"));
    assert!(p.contains("session: s1"));
}

#[test]
fn prompt_handles_no_gate_fire() {
    let p = assemble_reflection_prompt(&[bash("ls -la")], "", "s1");
    assert!(p.contains("(no gate fired)"));
}

#[test]
fn parse_accepts_a_well_formed_loosen_proposal() {
    let r = parse_proposal(
        "destructive_cli_guard|loosen|scratch temp dirs are safe to rm -rf",
        "s1",
    )
    .expect("a well-formed proposal must parse");
    assert_eq!(r.gate, "destructive_cli_guard");
    assert_eq!(r.edit, EditKind::Loosen);
    assert_eq!(r.from_session, "s1");
    assert!(r.rationale.contains("scratch"));
}

#[test]
fn parse_accepts_tighten() {
    let r = parse_proposal("pii_data_guard|tighten|missed an email in a log line", "s2").unwrap();
    assert_eq!(r.edit, EditKind::Tighten);
}

#[test]
fn parse_rejects_unknown_gate() {
    // A hallucinated gate name must NOT yield a proposal (fail-closed).
    assert!(parse_proposal("totally_made_up_guard|loosen|whatever", "s1").is_none());
}

#[test]
fn parse_rejects_malformed_shape() {
    assert!(parse_proposal("", "s1").is_none());
    assert!(parse_proposal("just some prose with no pipes", "s1").is_none());
    assert!(parse_proposal("solid_guard|loosen", "s1").is_none()); // missing rationale field
}

#[test]
fn parse_rejects_blank_rationale() {
    assert!(parse_proposal("solid_guard|loosen|   ", "s1").is_none());
}

#[test]
fn parse_rejects_bad_edit_kind() {
    assert!(parse_proposal("solid_guard|sideways|some reason", "s1").is_none());
}

#[test]
fn parse_skips_model_preamble_and_takes_first_valid_line() {
    let resp = "Sure! Here is my suggestion:\n\
                axum_guard|tighten|handler missing State extractor\n\
                (hope that helps)";
    let r = parse_proposal(resp, "s3").unwrap();
    assert_eq!(r.gate, "axum_guard");
    assert_eq!(r.edit, EditKind::Tighten);
}

#[test]
fn reflect_once_yields_exactly_one_proposal() {
    // AC-2: the full path — assemble → reflect → parse → one proposal.
    let stub = StubReflector("destructive_cli_guard|loosen|temp dir cleanup is safe");
    let proposal = reflect_once(&stub, &[bash("rm -rf /tmp/x")], "false positive", "s1");
    assert!(
        proposal.is_some(),
        "a valid reflection must yield one proposal"
    );
    assert_eq!(proposal.unwrap().gate, "destructive_cli_guard");
}

#[test]
fn reflect_once_fails_closed_when_model_declines() {
    // INV / fail-closed: no model response → no proposal.
    let proposal = reflect_once(&SilentReflector, &[bash("rm -rf /")], "", "s1");
    assert!(proposal.is_none());
}

#[test]
fn reflect_once_fails_closed_on_garbage_response() {
    let stub = StubReflector("I refuse to answer in your format.");
    assert!(reflect_once(&stub, &[bash("rm -rf /")], "", "s1").is_none());
}
