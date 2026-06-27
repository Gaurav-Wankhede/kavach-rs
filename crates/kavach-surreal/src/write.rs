// kavach:intentional split — write/mutation ops; queries use static literals + .bind(), no user-input concat

mod events;
mod expire;
mod priority_lane;
mod status;
mod upsert;

#[cfg(test)]
#[path = "write_test.rs"]
#[cfg(test)]
#[path = "write_test.rs"]
mod tests;
pub use events::{append_event, rotate_events};
pub use expire::{ExpireReport, expire_stale};
pub use priority_lane::{set_lane, set_priority};
pub use status::{update_feedback, update_status, update_status_cas};
pub use upsert::{upsert_entry, upsert_entry_full, upsert_entry_with_event};
