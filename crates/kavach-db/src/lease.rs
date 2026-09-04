// SPEC: docs/architecture/session-occupancy-lease.md
// Hub module for the session-occupancy lease primitive (lease/TTL + heartbeat + fencing token).
mod acquire;
mod acquire_set;
mod heartbeat;
mod reclaim;
mod recovery;
mod renew;
mod status;
mod types;
mod unlock;

pub use acquire::acquire;
pub use acquire_set::{AcquireSetOutcome, acquire_set};
pub use heartbeat::heartbeat;
pub use reclaim::reclaim_orphaned_in_progress;
pub use recovery::clear_stale_for_session;
pub use renew::{RENEW_INTERVAL_SECS, renew_active_leases};
pub use status::status;
pub use types::{AcquireOutcome, LEASE_TTL_SECS, Lease};
pub use unlock::unlock;

#[cfg(test)]
mod types_tests;
