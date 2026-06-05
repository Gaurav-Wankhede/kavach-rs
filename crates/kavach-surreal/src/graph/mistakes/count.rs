use crate::error::Result;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb_types::SurrealValue;

/// Queries the hit count for an anti-pattern.
///
/// # Errors
/// Propagates `Error::Surreal` when the query fails.
pub async fn query_anti_pattern_hit_count(
    db: &Surreal<Db>,
    anti_pattern_name: &str,
) -> Result<i64> {
    #[derive(SurrealValue)]
    struct Row {
        n: i64,
    }

    let q = "SELECT count(<-instance_of<-entity) AS n \
             FROM entity WHERE entity_type = 'anti_pattern' AND name = $name \
             LIMIT 1";
    let mut resp = db
        .query(q)
        .bind(("name", anti_pattern_name.to_owned()))
        .await?;
    let row: Option<Row> = resp.take(0)?;
    row.map_or(Ok(0), |r| Ok(r.n))
}
