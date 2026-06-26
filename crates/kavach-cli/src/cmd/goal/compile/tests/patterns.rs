// Per-pattern emit proofs for the five patterns added by the harness
// enhancement. THE ORACLE: each pattern emits its distinct workflow.js shape,
// untrusted widths are clamped, and every pattern is runnable-shaped (meta +
// return + goal_id).
use super::common::with_harness;
use crate::cmd::goal::compile::to_workflow_js;
use crate::cmd::goal::loop_yaml::Harness;

// --- Pattern 1 — Classify and Act (routing). ---

#[test]
fn classify_act_emits_routes_and_classifier() {
    let js = to_workflow_js(&with_harness(Harness::ClassifyAct {
        routes: vec!["fixer".into(), "answerer".into()],
    }));
    assert!(js.contains("name: 'classify-act'"), "got:\n{js}");
    assert!(js.contains("\"fixer\"") && js.contains("\"answerer\""));
    assert!(js.contains("phase: 'Classify'") && js.contains("phase: 'Act'"));
}

// --- Pattern 2 — Fan Out and Synthesize (parallelization). ---

#[test]
fn fan_out_emits_parallel_barrier_and_shard_count() {
    let js = to_workflow_js(&with_harness(Harness::FanOutSynthesize { shards: 4 }));
    assert!(js.contains("name: 'fan-out-synthesize'"), "got:\n{js}");
    assert!(js.contains("const SHARDS = 4"));
    assert!(js.contains("await parallel("));
}

#[test]
fn fan_out_clamps_runaway_shard_count() {
    let js = to_workflow_js(&with_harness(Harness::FanOutSynthesize { shards: 99_999 }));
    assert!(
        js.contains("const SHARDS = 64"),
        "shards not clamped:\n{js}"
    );
}

// --- Pattern 3 — Worker and Critic (evaluator-optimizer). ---

#[test]
fn worker_critic_emits_majority_vote() {
    let js = to_workflow_js(&with_harness(Harness::WorkerCritic { critics: 3 }));
    assert!(js.contains("name: 'worker-critic'"), "got:\n{js}");
    assert!(js.contains("const CRITICS = 3"));
    assert!(js.contains("approvals * 2 > verdicts.length"));
}

// --- Pattern 4 — Generate and Filter (voting). ---

#[test]
fn generate_filter_emits_pipeline_and_dedup() {
    let js = to_workflow_js(&with_harness(Harness::GenerateFilter { candidates: 8 }));
    assert!(js.contains("name: 'generate-filter'"), "got:\n{js}");
    assert!(js.contains("const CANDIDATES = 8"));
    assert!(js.contains("await pipeline("));
    assert!(js.contains("new Set()"));
}

// --- Pattern 5 — Pairwise Tournament. ---

#[test]
fn pairwise_tournament_emits_bracket_loop() {
    let js = to_workflow_js(&with_harness(Harness::PairwiseTournament {
        competitors: 4,
    }));
    assert!(js.contains("name: 'pairwise-tournament'"), "got:\n{js}");
    assert!(js.contains("const COMPETITORS = 4"));
    assert!(js.contains("while (bracket.length > 1)"));
}

#[test]
fn pairwise_tournament_floors_competitors_at_two() {
    let js = to_workflow_js(&with_harness(Harness::PairwiseTournament {
        competitors: 1,
    }));
    assert!(js.contains("const COMPETITORS = 2"), "not floored:\n{js}");
}

// --- Every pattern is valid, runnable-shaped JS: meta + a return. ---

// --- Model tiering: doers pin the cheap model, brains inherit the frontier. ---

#[test]
fn fan_out_tiers_shards_cheap_and_synthesis_frontier() {
    // Shards (doers) pin the cheap model; the synthesis (brain) does NOT — it
    // inherits the session frontier model. This is the token-saving contract.
    let js = to_workflow_js(&with_harness(Harness::FanOutSynthesize { shards: 4 }));
    // The FanOut agent line carries the cheap model.
    assert!(
        js.contains("phase: 'FanOut', model: 'claude-haiku-4-5'"),
        "shards must pin the cheap model:\n{js}"
    );
    // The Synthesize agent line carries NO model (frontier-inherited).
    assert!(
        js.contains("phase: 'Synthesize' }"),
        "synthesis must not pin a model:\n{js}"
    );
    assert!(
        !js.contains("phase: 'Synthesize', model:"),
        "synthesis must inherit frontier, not pin a model:\n{js}"
    );
}

#[test]
fn worker_is_cheap_and_critic_is_frontier() {
    let js = to_workflow_js(&with_harness(Harness::WorkerCritic { critics: 3 }));
    assert!(
        js.contains("phase: 'Work', model: 'claude-haiku-4-5'"),
        "worker cheap:\n{js}"
    );
    // Critic is an adversarial judge → brain → no model pin (keeps its schema).
    assert!(
        !js.contains("phase: 'Critique', model:"),
        "critic must be frontier:\n{js}"
    );
}

#[test]
fn every_pattern_declares_meta_and_returns() {
    let patterns = [
        Harness::ClassifyAct {
            routes: vec!["x".into()],
        },
        Harness::FanOutSynthesize { shards: 2 },
        Harness::WorkerCritic { critics: 2 },
        Harness::GenerateFilter { candidates: 2 },
        Harness::PairwiseTournament { competitors: 2 },
        Harness::LoopUntilDone,
    ];
    for h in patterns {
        let js = to_workflow_js(&with_harness(h.clone()));
        assert!(js.contains("export const meta ="), "no meta for {h:?}");
        assert!(js.contains("return {"), "no return for {h:?}");
        assert!(js.contains("goal_id: GOAL_ID"), "no goal_id for {h:?}");
    }
}
