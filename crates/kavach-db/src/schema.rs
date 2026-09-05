//! Schema hub. The single source of truth for the SQLite DDL is
//! `turso::SCHEMA_DDL` + the per-module DDL slices, applied idempotently at
//! `TursoStore::open` / `open_memory` via `init_schema`. `apply_schema` exists
//! for the explicit `--apply-schema` path and re-runs that same idempotent DDL.
mod core;
mod graph;
mod memory;
mod migrations;
#[cfg(test)]
#[path = "schema_test.rs"]
mod tests;
use crate::error::Result;
use crate::turso::TursoStore;
/// Re-applies the unified SQLite schema DDL (idempotent — every statement uses
/// `IF NOT EXISTS`).
///
/// # Errors
/// Propagates errors from the `Turso` query execution.
pub async fn apply_schema(db: &TursoStore) -> Result<()> {
    db.apply_schema().await.map_err(Into::into)
}
