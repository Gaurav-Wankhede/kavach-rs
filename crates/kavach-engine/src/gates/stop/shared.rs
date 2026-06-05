//! Shared substrate for the stop-gate guard microservices. Each child file is a
//! single reusable concern; guards compose them via these re-exports. One
//! source of truth per primitive — no duplication across the guard tree.

mod advisory;
mod breaker;
mod ctx;
mod focus;

pub(crate) use advisory::get_scope_advisory;
pub(crate) use breaker::should_block_behavioral;
pub(crate) use ctx::StopCtx;
pub(crate) use focus::{card_owns_any_turn_file, user_focus_supremacy_active};
