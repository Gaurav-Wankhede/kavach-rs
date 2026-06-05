// RPC methods for the autonomous harness loop (L2): a roadmap card carries the
// AI-chosen dynamic-workflow pattern + compiled `workflow.js` path, and the
// oracle's latest verdict is read back so the stop gate decides pass / retry.
mod get;
mod read;
mod resolve;
mod set;

pub use get::{GetHarnessParams, GetHarnessResult, get_harness};
pub use read::{LatestAttemptParams, LatestAttemptResult, latest_goal_attempt};
pub use set::{SetHarnessParams, SetHarnessResult, set_harness};
