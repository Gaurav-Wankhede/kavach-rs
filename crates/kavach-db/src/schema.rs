//! Schema hub. The single source of truth for the SQLite DDL is
//! `database::SCHEMA_DDL` + the per-module DDL slices, applied idempotently at
//! `DatabaseStore::open` / `open_memory` via `init_schema`.
//!
//! `apply_schema` exists for the explicit `--apply-schema` path and re-runs
//! that same idempotent DDL.
mod core;
mod graph;
mod memory;
mod migrations;
#[cfg(test)]
#[path = "schema_test.rs"]
mod tests;
use crate::error::Result;
use crate::db::DbStore;
/// Re-applies the unified SQLite schema DDL (idempotent — every statement uses
/// `IF NOT EXISTS`).
///
/// # Errors
/// Propagates errors from the `Database` query execution.
pub async fn apply_schema(db: &DbStore) -> Result<()> {
    db.apply_schema().await.map_err(Into::into)
}
