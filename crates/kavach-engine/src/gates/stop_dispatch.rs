//! Stop-gate dispatch + RPC helpers (extracted from stop.rs).
//!
//! This is the STATE layer of the stop gate: every function here answers a
//! kanban-state question via the RPC daemon (next task / hunt / backlog, claim,
//! done-card auto-verify) or computes a pure dispatch predicate. The
//! orchestrator `stop::run` composes these; keeping them in their own module
//! makes the authoritative state-driven path testable and small, separate from
//! the ordered guard chain in stop.rs.
mod card;
mod daemon;
mod query;
mod verify;

#[cfg(test)]
mod tests;

pub(crate) use card::{
    SOURCE_DOWN_KEY, card_entry_status, card_is_still_open, claim_card, is_backlog_saturated,
    live_lease_holder,
};
pub(crate) use query::{
    get_next_backlog_info, get_next_hunt_info, get_next_task_info, open_set_census,
};
pub(crate) use verify::{AutoVerify, auto_verify_done_cards};
