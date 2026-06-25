//! Detects distributed-systems anti-patterns: missing timeouts, unjittered retries, sync fanout, unbounded queues, missing idempotency, missing circuit-breaker, cache-as-bandaid.

mod detect;
mod types;
mod util;

pub use detect::detect;
pub use types::{SysSeverity, SysViolation};

#[cfg(test)]
#[path = "system_design_guard/tests.rs"]
mod tests;
