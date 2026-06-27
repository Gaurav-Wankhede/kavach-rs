//! Schema hub: applies the per-concern SurrealQL DDL nano-files in dependency order.
mod core;
mod graph;
mod memory;
mod migrations;

#[cfg(test)]
#[path = "schema_test.rs"]
#[cfg(test)]
#[path = "schema_test.rs"]
mod tests;
use crate::error::Result;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;

// Order matters: migrations' FTS indexes + backfills reference tables defined in earlier blocks.
const SCHEMA_PARTS: [&str; 4] = [core::DDL, memory::DDL, graph::DDL, migrations::DDL];

/// Applies the schema DDL to the `SurrealDB` instance.
///
/// # Errors
/// Propagates errors from the `SurrealDB` query execution.
pub async fn apply_schema(db: &Surreal<Db>) -> Result<()> {
    db.query(SCHEMA_PARTS.concat()).await?;
    Ok(())
}
