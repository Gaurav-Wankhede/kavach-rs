// split: intentional - durable harness runtime-state store (session_runtime table)
// sql-safe: queries use static literals + .bind() for params, no user input concatenation
use crate::error::Result;
use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb_types::SurrealValue;

/// One `session_runtime` row: the full `SessionState` serialized into `state_blob`.
///
/// Keyed by `session_id` so a new conversation (new `session_id`)
/// cannot read a prior conversation's row — the rehydration drift that lets
/// stale `files_modified` / `research_done` leak across a `/clear`.
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[non_exhaustive]
pub struct SessionRuntimeRow {
    pub session_id: String,
    pub workdir: String,
    pub state_blob: String,
}

/// Fetch the runtime row for exactly this `session_id`.
/// `None` ⇒ no row for this session — caller must start fresh, never inherit
/// another session's state.
///
/// # Errors
/// Propagates `Error::Surreal` when the database query fails.
pub async fn session_get_by_id(
    db: &Surreal<Db>,
    session_id: &str,
) -> Result<Option<SessionRuntimeRow>> {
    let query = "SELECT session_id, workdir, state_blob FROM session_runtime \
                 WHERE session_id = $sid LIMIT 1";
    let mut response = db.query(query).bind(("sid", session_id.to_owned())).await?;
    let row: Option<SessionRuntimeRow> = response.take(0)?;
    Ok(row)
}

/// Idempotent upsert of a session's runtime state.
///
/// The `idx_session_runtime_sid` UNIQUE index makes `session_id` the natural key;
/// UPSERT ... WHERE keeps one row per session and refreshes `updated_at` on every write-through.
///
/// # Errors
/// Propagates `Error::Surreal` when the database query fails.
pub async fn session_upsert(
    db: &Surreal<Db>,
    session_id: &str,
    workdir: &str,
    state_blob: &str,
) -> Result<()> {
    let query = "UPSERT session_runtime \
                 SET session_id = $sid, workdir = $workdir, \
                     state_blob = $state_blob, updated_at = time::now() \
                 WHERE session_id = $sid";
    db.query(query)
        .bind(("sid", session_id.to_owned()))
        .bind(("workdir", workdir.to_owned()))
        .bind(("state_blob", state_blob.to_owned()))
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::open_memory;
    use crate::schema::apply_schema;

    #[tokio::test]
    async fn upsert_then_get_round_trips() {
        let db = open_memory().await.expect("open mem db");
        apply_schema(&db).await.expect("schema");
        session_upsert(&db, "sess_a", "/tmp/wd", "blob-one")
            .await
            .expect("upsert");
        let row = session_get_by_id(&db, "sess_a").await.expect("get");
        let row = row.expect("row present");
        assert_eq!(row.session_id, "sess_a");
        assert_eq!(row.state_blob, "blob-one");
    }

    #[tokio::test]
    async fn upsert_is_idempotent_one_row_per_session() {
        let db = open_memory().await.expect("open mem db");
        apply_schema(&db).await.expect("schema");
        session_upsert(&db, "sess_b", "/tmp/wd", "v1")
            .await
            .expect("upsert v1");
        session_upsert(&db, "sess_b", "/tmp/wd", "v2")
            .await
            .expect("upsert v2");
        let row = session_get_by_id(&db, "sess_b")
            .await
            .expect("get")
            .expect("row");
        // Second upsert overwrites — the UNIQUE index keeps exactly one row.
        assert_eq!(row.state_blob, "v2");
    }

    #[tokio::test]
    async fn get_unknown_session_returns_none() {
        let db = open_memory().await.expect("open mem db");
        apply_schema(&db).await.expect("schema");
        let row = session_get_by_id(&db, "sess_never_written")
            .await
            .expect("get");
        assert!(
            row.is_none(),
            "absent session_id must yield None, not a stale row"
        );
    }
}
