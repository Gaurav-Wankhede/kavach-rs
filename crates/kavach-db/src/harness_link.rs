use crate::db::DbStore;
// Autonomous harness loop — write/read the harness link on a roadmap card.
//
// L2 of the DB-driven harness loop: a card carries the dynamic-workflow
// `harness` pattern the AI chose plus the path to its compiled `workflow.js`.
// `set_harness` is the write side (mirrors `write::set_priority`, roadmap-only);
// `latest_goal_attempt` is the read side the stop gate uses to decide the loop.
//
// SOURCE: decision.goal-harness-6-patterns · roadmap.unit.harness-loop-L2-rpc.
use crate::error::{Error, Result};

/// The most recent `goal_loop_attempt` event for a project.
///
/// The verdict the oracle wrote on the last harness run. `payload` is the raw
/// JSON the oracle emitted (verdict, attempt count, exit code); the stop gate
/// parses it to decide pass / retry / escalate.
#[derive(Debug)]
#[non_exhaustive]
pub struct GoalAttempt {
    pub id: String,
    pub payload: Option<serde_json::Value>,
}

/// Set (or clear) the `harness` pattern + compiled `workflow_path` on a card.
///
/// Roadmap-only — other categories have no harness column. Passing `None`
/// clears the field (card falls back to ordinary kanban dispatch).
///
/// # Errors
/// `RecordNotFound` if the (project, key) row is absent or category isn't
/// roadmap; propagates `Error::Database` from the UPDATE.
pub async fn set_harness(
    db: &DbStore,
    project_id: &str,
    entry_key: &str,
    harness: Option<&str>,
    workflow_path: Option<&str>,
) -> Result<String> {
    let now = crate::db::now_rfc3339();
    let mut response = db
        .exec_params(
            "UPDATE memories SET harness = $harness, workflow_path = $wf, updated_at = $now \
             WHERE project = $pid AND category = 'roadmap' AND entry_key = $key \
             RETURNING entry_key",
            libsql::params![
                harness.map(str::to_owned),
                workflow_path.map(str::to_owned),
                now,
                project_id.to_owned(),
                entry_key.to_owned()
            ],
        )
        .await?;
    match response.next().await? {
        Some(r) => Ok(r.get::<String>(0)?),
        None => Err(Error::RecordNotFound(format!(
            "no roadmap row for key {entry_key} to set harness"
        ))),
    }
}

/// Read the latest `goal_loop_attempt` event for `project_id`, newest first.
/// Returns `None` when no attempt has been recorded yet (the loop hasn't run).
///
/// # Errors
/// Propagates `Error::Database` from the SELECT.
pub async fn latest_goal_attempt(db: &DbStore, project_id: &str) -> Result<Option<GoalAttempt>> {
    let mut rows = db
        .exec_params(
            "SELECT id, payload FROM events \
             WHERE kind = 'goal_loop_attempt' AND project = $pid \
             ORDER BY created_at DESC LIMIT 1",
            libsql::params![project_id.to_owned()],
        )
        .await?;
    match rows.next().await? {
        Some(r) => {
            let payload = r
                .get::<Option<String>>(1)?
                .and_then(|s| serde_json::from_str(&s).ok());
            Ok(Some(GoalAttempt {
                id: r.get::<String>(0)?,
                payload,
            }))
        }
        None => Ok(None),
    }
}

/// A roadmap card's harness link: the pattern + compiled `workflow_path` the
/// stop gate (L3) reads to decide whether to auto-run a workflow. Both `None`
/// for ordinary cards (no harness assigned).
#[derive(Debug)]
#[non_exhaustive]
pub struct HarnessLink {
    pub harness: Option<String>,
    pub workflow_path: Option<String>,
}

/// Read the `harness` + `workflow_path` columns for one roadmap card.
/// `None` when the (project, key) row is absent.
///
/// # Errors
/// Propagates `Error::Database` from the SELECT.
pub async fn get_harness_link(
    db: &DbStore,
    project_id: &str,
    entry_key: &str,
) -> Result<Option<HarnessLink>> {
    let mut rows = db
        .exec_params(
            "SELECT harness, workflow_path FROM memories \
             WHERE project = $pid AND category = 'roadmap' AND entry_key = $key LIMIT 1",
            libsql::params![project_id.to_owned(), entry_key.to_owned()],
        )
        .await?;
    match rows.next().await? {
        Some(r) => Ok(Some(HarnessLink {
            harness: r.get::<Option<String>>(0)?,
            workflow_path: r.get::<Option<String>>(1)?,
        })),
        None => Ok(None),
    }
}
