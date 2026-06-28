//! Audit lenses — one detector family per file, all returning unified Findings.
pub(super) mod security;
pub(super) mod silent_fail;
pub(super) mod worst_practice;
pub(super) mod yagni;
