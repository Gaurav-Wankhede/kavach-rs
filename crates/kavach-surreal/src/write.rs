// split: intentional - write/mutation operations on typed memory tables
// sql-safe: queries use static literals + .bind() for params, no user input concatenation

mod expire;
mod events;
mod priority_lane;
mod status;
mod upsert;

#[cfg(test)]
#[path = "write/tests.rs"]
mod tests;

pub use expire::{expire_stale, ExpireReport};
pub use events::{append_event, rotate_events};
pub use priority_lane::{set_lane, set_priority};
pub use status::{update_feedback, update_status, update_status_cas};
pub use upsert::{upsert_entry, upsert_entry_full, upsert_entry_with_event};

pub(crate) use events::EventRow;
pub(crate) use status::UpdatedIdRow;
