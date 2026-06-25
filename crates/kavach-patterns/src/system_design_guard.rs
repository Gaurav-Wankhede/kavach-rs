//! Distributed-systems anti-pattern detector: timeouts, jitter, fanout, idempotency, circuit-breaker, cache-TTL.

mod detect;
mod types;
mod util;

pub use detect::detect;
pub use types::{SysSeverity, SysViolation};

#[cfg(test)]
#[path = "system_design_guard/tests.rs"]
mod tests;
