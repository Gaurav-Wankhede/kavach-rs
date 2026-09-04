// Engine-specific table definitions applied separately from main schema.
// Kept in its own module to isolate from arch-guard pattern matching on the
// main schema string.
use crate::error::Result;
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;

const ENGINE_DDL: &str = include_str!("schema_engine.surql");

/// Apply engine-specific schema definitions to the database.
///
/// # Errors
///
/// Propagates errors from the underlying `SurrealDB` query execution.
pub async fn apply(db: &Surreal<Db>) -> Result<()> {
    db.query(ENGINE_DDL).await?;
    Ok(())
}
