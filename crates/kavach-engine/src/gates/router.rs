//! Gate-severity router with per-turn budget circuit-breaker.
//!
//! Central dispatch: every gate calls `router::emit(severity, msg)` instead of
//! reaching for `kavach_hook::exit_pre_tool_*` directly. The budget caps the
//! "stack of blocks" anti-pattern (≥10 fires in a turn → silent downgrade).
//! SOURCE: roadmap.unit.gate-severity-router · pixelmojo 2026 quality-loop.
mod budget;
mod dispatch;
#[cfg(test)]
#[path = "router_test.rs"]
#[cfg(test)]
#[path = "router_test.rs"]
mod tests;
pub(crate) use budget::{observe_tool_call, reset_for_new_turn};
pub(crate) use dispatch::emit;
