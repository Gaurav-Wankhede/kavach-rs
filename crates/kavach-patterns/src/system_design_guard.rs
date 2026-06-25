//! System Architecture & System Design at Scale Gate
//!
//! Detects distributed-systems anti-patterns that cause cascading failures
//! at scale: missing timeouts, unjittered retries, sync fanout, unbounded
//! queues, missing idempotency, missing circuit-breaker, cache-as-bandaid.
//!
//! SOURCES (verified 2026-05):
//! - <https://temporal.io/blog/error-handling-in-distributed-systems>
//! - <https://arxiv.org/html/2512.16959v1>
//! - <https://system-design.space/en/chapter/resilience-patterns>/
//! - <https://www.ceamkrier.com/post/resilient-distributed-systems-saga-circuit-breaker-idempotency>/
//! - <https://designgurus.substack.com/p/7-system-design-anti-patterns-that>
//! - <https://vfunction.com/blog/how-to-avoid-microservices-anti-patterns>/
//! - <https://distributedsystemauthority.com/circuit-breaker-pattern>

mod detect;
mod types;
mod util;

pub use detect::detect;
pub use types::{SysSeverity, SysViolation};

#[cfg(test)]
#[path = "system_design_guard/tests.rs"]
mod tests;
