// split: intentional - durable harness runtime-state store (session_runtime table)
// sql-safe: queries use static literals + params, no user input concatenation
use crate::error::Result;
use crate::db::DbStore;
use serde::{Deserialize, Serialize};

/// One `session_runtime` row: the full `SessionState` serialized into `state_blob`.
///
/// Keyed by `session_id` so a new conversation (new `session_id`)
/// cannot read a prior conversation's row — the rehydration drift that lets
/// stale `files_modified` / `research_done` leak across a `/clear`.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
/// Propagates `Error::Database` when the database query fails.
pub async fn session_get_by_id(
    db: &DbStore,
    session_id: &str,
) -> Result<Option<SessionRuntimeRow>> {
    let mut rows = db
        .exec_params(
            "SELECT session_id, workdir, state_blob FROM session_runtime \
             WHERE session_id = $sid LIMIT 1",
            libsql::params![session_id.to_owned()],
        )
        .await?;
    match rows.next().await? {
        Some(r) => Ok(Some(SessionRuntimeRow {
            session_id: r.get::<String>(0)?,
            workdir: r.get::<String>(1)?,
            state_blob: r.get::<String>(2)?,
        })),
        None => Ok(None),
    }
}

/// Idempotent upsert of a session's runtime state.
///
/// `session_id` is the PRIMARY KEY; `ON CONFLICT DO UPDATE` keeps one row per
/// session and refreshes `updated_at` on every write-through.
///
/// # Errors
/// Propagates `Error::Database` when the database query fails.
pub async fn session_upsert(
    db: &DbStore,
    session_id: &str,
    workdir: &str,
    state_blob: &str,
) -> Result<()> {
    let now = crate::db::now_rfc3339();
    db.exec_params(
        "INSERT INTO session_runtime (session_id, workdir, state_blob, updated_at) \
         VALUES ($sid, $workdir, $state_blob, $now) \
         ON CONFLICT(session_id) DO UPDATE SET \
             workdir = excluded.workdir, state_blob = excluded.state_blob, \
             updated_at = excluded.updated_at",
        libsql::params![
            session_id.to_owned(),
            workdir.to_owned(),
            state_blob.to_owned(),
            now
        ],
    )
    .await?;
    Ok(())
}
