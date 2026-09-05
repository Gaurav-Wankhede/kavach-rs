// SOURCE: relocated from write/upsert.rs — roadmap.upsert-microfile-split (kavach:relocated)
use crate::error::Result;

/// Append entity + project + reference INSERT statements to the txn.
///
/// Entities are upserted by id (`memory:<qname>`, `project:<pname>`,
/// `skill:<ref>`); edges are inserted with `ON CONFLICT DO NOTHING` so re-runs
/// never duplicate. Ids are built with `||` concatenation — a `?N` placeholder
/// inside a string literal is literal text, not a bound parameter.
pub(super) async fn append_entity_graph(
    conn: &libsql::Connection,
    qualified_name: &str,
    project_name: &str,
    references: &[String],
) -> Result<()> {
    if qualified_name.is_empty() {
        return Ok(());
    }
    let now = crate::turso::now_rfc3339();
    // `Connection::query` defers statement errors into the returned Rows (they
    // surface only on `next()`); these are INSERTs, so use `execute`, which
    // returns the error immediately (§no_silent_failure).
    conn.execute(
        "INSERT INTO entities (id, entity_type, name, project) \
         VALUES ('memory:' || ?1, 'memory', ?1, NULL) \
         ON CONFLICT(id) DO NOTHING",
        libsql::params![qualified_name.to_owned()],
    )
    .await?;
    conn.execute(
        "INSERT INTO entities (id, entity_type, name, project) \
         VALUES ('project:' || ?1, 'project', ?1, NULL) \
         ON CONFLICT(id) DO NOTHING",
        libsql::params![project_name.to_owned()],
    )
    .await?;
    conn.execute(
        "INSERT INTO entity_edges (from_id, rel_type, to_id, weight, created_at) \
         VALUES ('memory:' || ?1, 'in_scope', 'project:' || ?2, 1.0, ?3) \
         ON CONFLICT(from_id, rel_type, to_id) DO NOTHING",
        libsql::params![
            qualified_name.to_owned(),
            project_name.to_owned(),
            now.clone()
        ],
    )
    .await?;
    for r in references {
        conn.execute(
            "INSERT INTO entities (id, entity_type, name, project) \
             VALUES ('skill:' || ?1, 'skill', ?1, NULL) \
             ON CONFLICT(id) DO NOTHING",
            libsql::params![r.to_owned()],
        )
        .await?;
        conn.execute(
            "INSERT INTO entity_edges (from_id, rel_type, to_id, weight, created_at) \
             VALUES ('memory:' || ?2, 'references', 'skill:' || ?1, 1.0, ?3) \
             ON CONFLICT(from_id, rel_type, to_id) DO NOTHING",
            libsql::params![r.to_owned(), qualified_name.to_owned(), now.clone()],
        )
        .await?;
    }
    Ok(())
}
