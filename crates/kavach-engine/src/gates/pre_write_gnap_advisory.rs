//! GNAP spec advisory gate — injects RFC 9635/9767 reference on auth-related writes.
//!
//! Trigger: PreToolUse:Write|Edit where `file_path` contains auth/grant/token
//! patterns. Output: `[GNAP_SPEC_REF]` advisory block with wire formats + types.
//! `detect` classifies relevance; `extract` slices the spec by concept; the
//! `advisory` orchestrator loads the spec and emits the block.
mod advisory;
mod detect;
mod extract;
#[cfg(test)]
#[path = "pre_write_gnap_advisory_test.rs"]
mod tests;
pub(crate) use advisory::advisory;
