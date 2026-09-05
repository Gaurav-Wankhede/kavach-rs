use crate::error::Result;
use crate::turso::TursoStore;

/// Queries the hit count for an anti-pattern (inbound `instance_of` edges).
///
/// # Errors
/// Propagates `Error::Turso` when the query fails.
pub async fn query_anti_pattern_hit_count(db: &TursoStore, anti_pattern_name: &str) -> Result<i64> {
    let mut resp = db
        .exec_params(
            "SELECT COUNT(*) FROM entity_edges ed \
             JOIN entities e ON e.id = ed.to_id \
             WHERE ed.rel_type = 'instance_of' AND e.entity_type = 'anti_pattern' \
               AND e.name = ?1",
            libsql::params![anti_pattern_name.to_owned()],
        )
        .await?;
    match resp.next().await? {
        Some(row) => Ok(row.get::<i64>(0)?),
        None => Ok(0),
    }
}
