//! kavach-ope — offline off-policy evaluation (Layer B of the harness-rl design).
//!
//! Given logged bandit samples `(action a, propensity p, reward r)` produced by
//! the live rule-gate (the logging policy), estimate the VALUE of a candidate
//! target policy WITHOUT deploying it. Estimators land in increasing robustness:
//!
//! - [`ips`] — Inverse Propensity Scoring: unbiased, high variance. [this wave]
//! - Direct Method (a learned reward model; low variance, biased) and
//!   Doubly-Robust (unbiased if EITHER model is right) follow in later waves.
//!
//! Pure functions over slices — no DB, no RPC. The caller (an RPC method or CLI)
//! deserializes `bandit_log` rows into [`LoggedSample`] and passes them in.
//!
//! WHY this is the deploy gate: a candidate gate policy ships ONLY if its
//! lower confidence bound beats the incumbent (design D4). The CI is therefore
//! load-bearing, not decoration — see [`Estimate::lower_confidence_bound`].
#![allow(
    clippy::float_arithmetic,
    reason = "this crate IS floating-point statistics (IPS/DR estimators + CIs); \
              the workspace restriction lint does not fit an estimator crate"
)]
mod sample;
pub use sample::{Action, LoggedSample};
mod estimate;
pub use estimate::Estimate;
pub mod audit;
pub mod controller;
pub mod dm;
pub mod doubly_robust;
pub mod explore;
pub mod ips;
pub mod label;
pub mod trust;
#[cfg(test)]
#[path = "lib_test.rs"]
mod tests;
