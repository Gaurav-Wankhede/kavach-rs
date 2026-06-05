// SPEC: docs/architecture/session-occupancy-lease.md
// Hub module for the session-occupancy lease primitive (lease/TTL + heartbeat + fencing token).
mod acquire;
mod heartbeat;
mod recovery;
mod status;
mod types;
mod unlock;

pub use acquire::acquire;
pub use heartbeat::heartbeat;
pub use recovery::clear_stale_for_session;
pub use status::status;
pub use types::{AcquireOutcome, LEASE_TTL_SECS, Lease};
pub use unlock::unlock;

#[cfg(test)]
mod types_tests;
