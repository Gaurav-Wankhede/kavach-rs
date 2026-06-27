//! RSCB-MC — the risk-sensitive contextual-bandit controller (Layer C).
//!
//! A lightweight hot-path policy over the fixed action set `{Allow, Ask, Block}`.
//!
//! Not an LLM, no rollouts, no hosted inference — just a pessimistic value
//! comparison over per-action estimates the OPE layer already produces.
//!
//! Two design invariants, both fail-closed (design C2 / the P5 reward-hacking
//! guard):
//! - PESSIMISTIC: each action is scored by its LOWER confidence bound
//!   (`value − z·std_error`), never its point estimate, so a high-variance lucky
//!   mean cannot win. Over-conservative by construction.
//! - ABSTENTION: `Ask` (defer to the human) is the safe action. When no action's
//!   pessimistic score clears a confidence floor, the controller abstains to
//!   `Ask` rather than guess.
//!
//! Scope (design D2): this tunes only ADVISORY gates. Hard P0/forbid gates bypass
//! the controller entirely and stay static — enforced structurally by
//! [`AdvisoryCandidates`], whose constructor a hard-block path cannot satisfy.
//!
//! Composed from leaves: `value` (`ActionValue` + `RiskConfig`), `scope` (the
//! advisory-only structural guard), `select` (pessimistic `choose`), `promote`
//! (the canary gate).
mod promote;
mod scope;
mod select;
mod value;
pub use promote::promote;
pub use scope::{AdvisoryCandidates, GateScope};
pub use select::choose;
pub use value::{ActionValue, RiskConfig};
#[cfg(test)]
#[path = "controller_test.rs"]
#[path = "controller_test.rs"]
mod tests;
