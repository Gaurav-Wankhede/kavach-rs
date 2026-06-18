// SPEC: docs/architecture/session-occupancy-lease.md §CLI surface
// RPC verbs over kavach_surreal::lease primitive (acquire/heartbeat/unlock/status).
// jsonrpsee 0.24 — https://docs.rs/jsonrpsee/0.24
mod acquire;
mod acquire_set;
mod heartbeat;
mod status;
mod unlock;

pub use acquire::{AcquireParams, AcquireResult, acquire};
pub use acquire_set::{AcquireSetParams, AcquireSetResult, AcquiredLease, acquire_set};
pub use heartbeat::{HeartbeatParams, HeartbeatResult, heartbeat};
pub use status::{StatusParams, StatusResult, status};
pub use unlock::{UnlockParams, UnlockResult, unlock};
