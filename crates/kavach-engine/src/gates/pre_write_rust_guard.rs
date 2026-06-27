//! Rust Production Guard — pre-write gate for `.rs` files.
//!
//! P0 violations (unwrap, `_var`, panic, as-cast) hard-block; P1 (clone,
//! catch-all, allow) are advisories; crate-root lint gaps are lint advisories.
//! `block` owns the P0 hard-block; `advisory` owns the P1 + lint advisories.
//! SOURCE: <https://doc.rust-lang.org/clippy/configuration.html>
mod advisory;
mod block;
#[cfg(test)]
#[path = "pre_write_rust_guard_test.rs"]
#[cfg(test)]
#[path = "pre_write_rust_guard_test.rs"]
mod tests;
pub(crate) use advisory::{format_advisory, format_lint_advisory};
pub(crate) use block::check;
