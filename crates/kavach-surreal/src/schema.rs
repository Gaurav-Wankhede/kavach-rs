//! Schema hub: applies the per-concern SurrealQL DDL nano-files in dependency order.
mod core;
mod graph;
mod memory;
mod migrations;

#[cfg(test)]
#[path = "schema/tests.rs"]
mod tests;

use crate::error::Result;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;

/// DDL concern-blocks applied in order: base tables, typed memory, graph/ops, then idempotent migrations (FTS indexes + backfills reference the tables defined earlier).
const SCHEMA_PARTS: [&str; 4] = [core::DDL, memory::DDL, graph::DDL, migrations::DDL];

/// Applies the schema DDL to the `SurrealDB` instance.
///
/// # Errors
/// Propagates errors from the `SurrealDB` query execution.
pub async fn apply_schema(db: &Surreal<Db>) -> Result<()> {
    db.query(SCHEMA_PARTS.concat()).await?;
    Ok(())
}
