// Model tiering for generated workflows — frontier brain plans/judges, cheap
// doers implement. Every emitter tags its `agent()` calls with a phase ROLE;
// this module renders the shared `MODELS` constant block + a `tierModel(role)`
// helper into the workflow header, so the JS picks the right tier per phase.
//
// The concrete model IDs mirror decision.harness.model-tiering (the single
// source of truth). They are constants here, NOT hand-typed at each call site,
// so a tier change is one edit. The directive (delegation_tiers): orchestrator/
// judge = FRONTIER (the brain, which the workflow runtime supplies as the
// session model); doers (implement/migrate/shard/candidate/worker) = CHEAP.
//
// SOURCE: decision.harness.model-tiering · global ~/.claude/CLAUDE.md delegation_tiers.
/// Cheap fast doer model — Claude Code's implementation tier (Haiku). A doer is
/// a doer even for shard/candidate/worker/critic-as-doer work.
pub(super) const CHEAP_MODEL: &str = "claude-haiku-4-5";
/// Phase role: which tier a phase's agents run at. Doer phases fan out cheap;
/// JUDGE/SYNTHESIS phases run on the frontier brain (the workflow's inherited
/// session model — we do NOT pin a frontier ID, so it always tracks the live
/// orchestrator model rather than a stale hardcode).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Role {
    /// Implementation/exploration doer — pinned to the cheap tier.
    Doer,
    /// Decision/judge/synthesis — inherits the frontier brain (no model pin).
    Brain,
}
/// The `opts` fragment for an `agent()` call given its phase + role. A `Doer`
/// gets `model: '<cheap>'`; a `Brain` gets no model key so it inherits the
/// session (frontier) model. Always includes the `phase` so the progress tree
/// groups correctly.
///
/// Returns a JS object-literal body WITHOUT braces, e.g. `phase: 'FanOut', model: 'claude-haiku-4-5'`
/// — callers wrap in `{{ ... }}`.
pub(super) fn agent_opts(phase: &str, role: Role) -> String {
    match role {
        Role::Doer => format!("phase: '{phase}', model: '{CHEAP_MODEL}'"),
        Role::Brain => format!("phase: '{phase}'"),
    }
}
#[cfg(test)]
#[path = "model_tier_test.rs"]
#[path = "model_tier_test.rs"]
mod tests;
