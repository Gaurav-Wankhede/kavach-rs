//! Response Security Guard — blocks privilege escalation via `serde(default)`.
//!
//! P0 (`block`): `#[serde(default)]` on a role/permission field enables
//! privilege escalation via omission. P1 (`advisory`): PII fields, missing
//! `deny_unknown_fields`, bool serde defaults.
mod advisory;
mod block;
#[cfg(test)]
#[path = "pre_write_response_guard_test.rs"]
#[cfg(test)]
#[path = "pre_write_response_guard_test.rs"]
mod tests;
pub(crate) use advisory::format_advisory;
pub(crate) use block::check;
