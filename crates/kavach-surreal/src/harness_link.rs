// Autonomous harness loop — write/read the harness link on a roadmap card.
//
// L2 of the DB-driven harness loop: a card carries the dynamic-workflow
// `harness` pattern the AI chose plus the path to its compiled `workflow.js`.
// `set_harness` is the write side (mirrors `write::set_priority`, roadmap-only);
// `latest_goal_attempt` is the read side the stop gate uses to decide the loop.
//
// SOURCE: decision.goal-harness-6-patterns · roadmap.unit.harness-loop-L2-rpc.
use crate::error::{Error, Result};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb_types::{RecordId, SurrealValue};

/// The most recent `goal_loop_attempt` event for a project.
///
/// The verdict the oracle wrote on the last harness run. `payload` is the raw
/// JSON the oracle emitted (verdict, attempt count, exit code); the stop gate
/// parses it to decide pass / retry / escalate.
#[derive(Debug, surrealdb_types::SurrealValue)]
#[non_exhaustive]
pub struct GoalAttempt {
    pub id: RecordId,
    pub payload: Option<surrealdb_types::Value>,
}

/// Set (or clear) the `harness` pattern + compiled `workflow_path` on a card.
///
/// Roadmap-only — other categories have no harness column. Passing `None`
/// clears the field (card falls back to ordinary kanban dispatch).
///
/// # Errors
/// `RecordNotFound` if the (project, key) row is absent or category isn't
/// roadmap; propagates `Error::Surreal` from the UPDATE.
pub async fn set_harness(
    db: &Surreal<Db>,
    project_id: &RecordId,
    entry_key: &str,
    harness: Option<&str>,
    workflow_path: Option<&str>,
) -> Result<RecordId> {
    const QUERY: &str = "UPDATE roadmap SET harness = $harness, \
         workflow_path = $wf, updated_at = time::now() \
         WHERE project = $pid AND entry_key = $key RETURN id";
    let mut response = db
        .query(QUERY)
        .bind(("harness", harness.map(str::to_owned)))
        .bind(("wf", workflow_path.map(str::to_owned)))
        .bind(("pid", project_id.clone()))
        .bind(("key", entry_key.to_owned()))
        .await?;
    let id: Option<RecordId> = response.take("id")?;
    id.ok_or_else(|| {
        Error::RecordNotFound(format!("no roadmap row for key {entry_key} to set harness"))
    })
}

/// Read the latest `goal_loop_attempt` event for `project_id`, newest first.
/// Returns `None` when no attempt has been recorded yet (the loop hasn't run).
///
/// # Errors
/// Propagates `Error::Surreal` from the SELECT.
pub async fn latest_goal_attempt(
    db: &Surreal<Db>,
    project_id: &RecordId,
) -> Result<Option<GoalAttempt>> {
    const QUERY: &str = "SELECT id, payload FROM event \
         WHERE event_type = 'goal_loop_attempt' AND project = $pid \
         ORDER BY created_at DESC LIMIT 1";
    let mut response = db.query(QUERY).bind(("pid", project_id.clone())).await?;
    let row: Option<GoalAttempt> = response.take(0)?;
    Ok(row)
}

/// A roadmap card's harness link: the pattern + compiled `workflow_path` the
/// stop gate (L3) reads to decide whether to auto-run a workflow. Both `None`
/// for ordinary cards (no harness assigned).
#[derive(Debug, SurrealValue)]
#[non_exhaustive]
pub struct HarnessLink {
    pub harness: Option<String>,
    pub workflow_path: Option<String>,
}

/// Read the `harness` + `workflow_path` columns for one roadmap card.
/// `None` when the (project, key) row is absent.
///
/// # Errors
/// Propagates `Error::Surreal` from the SELECT.
pub async fn get_harness_link(
    db: &Surreal<Db>,
    project_id: &RecordId,
    entry_key: &str,
) -> Result<Option<HarnessLink>> {
    const QUERY: &str = "SELECT harness, workflow_path FROM roadmap \
         WHERE project = $pid AND entry_key = $key LIMIT 1";
    let mut response = db
        .query(QUERY)
        .bind(("pid", project_id.clone()))
        .bind(("key", entry_key.to_owned()))
        .await?;
    let row: Option<HarnessLink> = response.take(0)?;
    Ok(row)
}
