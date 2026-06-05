//! Response Security Guard — blocks privilege escalation via `serde(default)`.
//!
//! P0 (`block`): `#[serde(default)]` on a role/permission field enables
//! privilege escalation via omission. P1 (`advisory`): PII fields, missing
//! `deny_unknown_fields`, bool serde defaults.
mod advisory;
mod block;

#[cfg(test)]
mod tests;

pub(crate) use advisory::format_advisory;
pub(crate) use block::check;
