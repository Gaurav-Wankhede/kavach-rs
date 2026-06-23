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
pub(crate) mod query;
pub(crate) mod verify;

#[cfg(test)]
mod tests;

pub(crate) use card::{
    SOURCE_DOWN_KEY, card_entry_status, card_is_still_open, claim_card, live_lease_holder,
};
pub(crate) use query::{
    census_rpc_only, disobedience_vocab_for, get_next_backlog_info, get_next_hunt_info,
    get_next_task_info, done_gaming_vocab_for, next_task_directive, next_task_rpc_only,
    open_set_census, oracle_config_for, reward_rubric_for,
};
pub(crate) use daemon::renew_my_leases;
pub(crate) use verify::{AutoVerify, auto_verify_done_cards};
