//! Rust 1.96+ / Edition 2024 Production Gate
//!
//! Dedicated guard for VERSION-SPECIFIC Rust patterns. Distinct from `rust_guard.rs`
//! (timeless anti-patterns). This file enforces Rust 1.96+ / Edition 2024 ONLY.
//!
//! SOURCES (verified 2026-05):
//! - <https://blog.rust-lang.org/2026/05/28/Rust-1.96.0>/
//! - <https://releases.rs/docs/1.96.0/>
//! - <https://blog.rust-lang.org/2026/04/16/Rust-1.95.0>/
//! - <https://doc.rust-lang.org/edition-guide/rust-2018/path-changes.html>
//! - <https://corrode.dev/blog/pitfalls-of-safe-rust>/
//! - <https://sherlock.xyz/post/rust-security-auditing-guide-2026>
mod detect;
mod detect_196;
mod patterns;
mod types;
pub use detect::detect;
pub use types::{Rust196Severity, Rust196Violation};
#[cfg(test)]
#[path = "rust_196_guard_test.rs"]
mod tests;
