//! Stage 2 enforcement: immutable migration ledger.
//!
//! Editing an applied sqlx migration breaks its checksum ledger (sqlx
//! Discussion #1292). This gate blocks such edits at write-time using a path
//! classifier plus a git-mtime / `.applied`-marker "presumed applied" heuristic
//! — no DB round-trip, sub-millisecond on the hot path.
//!
//! SOURCE: `decision:rca.immutable_migration_gate`;
//! <https://github.com/launchbadge/sqlx/discussions/1292>.
//! `classify` matches the migration path shape; `applied` is the heuristic;
//! `check` is the gate entry point. An env override exists for drift reconciliation.
mod applied;
mod check;
mod classify;
#[cfg(test)]
#[path = "pre_write_immutable_migration_test.rs"]
mod tests;
pub(crate) use check::check;
