//! Detects distributed-system anti-patterns: timeouts, jitter, fanout, idempotency, circuit-breaker.

mod detect;
mod types;
mod util;

pub use detect::detect;
pub use types::{SysSeverity, SysViolation};

#[cfg(test)]
#[path = "system_design_guard/tests.rs"]
mod tests;
