//! Tier-emission proofs: doers pin the cheap model, brains inherit the frontier
//! session model (no pin). Guards the token-saving contract of model-tiering.
use super::{CHEAP_MODEL, Role, agent_opts};

#[test]
fn doer_pins_the_cheap_model() {
    let opts = agent_opts("FanOut", Role::Doer);
    assert!(
        opts.contains(CHEAP_MODEL),
        "doer must pin cheap model: {opts}"
    );
    assert!(
        opts.contains("phase: 'FanOut'"),
        "doer keeps its phase: {opts}"
    );
}

#[test]
fn brain_inherits_frontier_no_model_pin() {
    // A judge/synthesis phase must NOT pin a model — it inherits the session
    // (frontier) model so it always tracks the live orchestrator, never a stale
    // hardcode. Absence of a `model:` key IS the contract.
    let opts = agent_opts("Synthesize", Role::Brain);
    assert!(
        !opts.contains("model:"),
        "brain must not pin a model: {opts}"
    );
    assert!(
        opts.contains("phase: 'Synthesize'"),
        "brain keeps its phase: {opts}"
    );
}

#[test]
fn cheap_model_matches_claude_code_doer_tier() {
    // Mirrors the haiku frontmatter on the doer agents + decision.harness.model-tiering.
    assert_eq!(CHEAP_MODEL, "claude-haiku-4-5");
}
