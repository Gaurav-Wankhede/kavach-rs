//! Async/Sync Pattern Guard — Tokio cancellation safety + runtime starvation.
//!
//! SOURCES (verified 2026-05):
//! - <https://docs.rs/tokio/latest/tokio/macro.select.html>
//! - <https://sunshowers.io/posts/cancelling-async-rust>/
//! - <https://rfd.shared.oxide.computer/rfd/0400> (cancel-safe-futures)
//! - <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>

mod detect;
mod rules;
mod tests;
mod types;
mod walk;

pub use detect::detect;
pub use types::{AsyncSeverity, AsyncViolation};
