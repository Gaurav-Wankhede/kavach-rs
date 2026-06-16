// Declarative source-of-truth for an oracle-gated goal loop (hub).
//
// Split into leaves to keep each file small: `oracle` (proof signal +
// max-attempts policy), `harness` (the six dynamic-workflow patterns), `limits`
// (loop-control brakes), `model` (the GoalLoopYaml struct + serde/disk I/O).
// Re-exports keep `super::loop_yaml::{...}` paths stable for the compiler.
//
// SOURCE: decision.goal-oracle-workflow · decision.goal-harness-6-patterns.
mod harness;
mod limits;
mod loophole;
mod model;
mod oracle;

#[cfg(test)]
mod tests;

pub(crate) use harness::Harness;
pub(crate) use limits::LoopLimits;
pub(crate) use loophole::{Lens, LoopholeIteration};
pub(crate) use model::GoalLoopYaml;
pub(crate) use oracle::{OnMaxAttempts, Oracle};
