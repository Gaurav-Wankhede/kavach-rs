//! Architecture Research Gate — enforces /arch skill for architectural decisions.
//!
//! Detects architectural patterns (scaling, caching, messaging, data layer, service patterns)
//! and requires structured documentation before allowing writes.

mod check;
mod detect;
mod triggers;
mod types;

pub use check::{advise, check};
pub use detect::{count_arch_fields, detect, has_arch_comment};
pub use types::{ArchFinding, ArchGuardOutcome, ArchScope};
#[cfg(test)]
#[path = "arch_guard_test.rs"]
#[cfg(test)]
mod tests;