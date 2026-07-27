//! Schema hub: applies the unified DDL (single source of truth for all 32 tables).
mod core;
mod graph;
mod memory;
mod migrations;
pub mod unified;
#[cfg(test)]
#[path = "schema_test.rs"]
mod tests;
use crate::error::Result;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
/// Applies the unified schema DDL to the `SurrealDB` instance.
///
/// Uses `unified::DDL` as the single source of truth for all 32 tables
/// (core + memory + graph + WebFind).
///
/// # Errors
/// Propagates errors from the `SurrealDB` query execution.
pub async fn apply_schema(db: &Surreal<Db>) -> Result<()> {
    db.query(unified::DDL).await?;
    Ok(())
}
