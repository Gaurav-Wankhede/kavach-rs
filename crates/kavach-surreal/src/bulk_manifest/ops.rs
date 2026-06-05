// split: intentional - cohesive bulk_manifest CRUD surface (create/get/bump/close/list)
// Async DB ops for bulk_manifest. I/O surface only — types in `types.rs`,
// SQL in `sql.rs`. All params arrive via `bind(("name", v))`; no format!.
// Errors propagate via `?` through `Error::Surreal` (#[from] surrealdb::Error)
// and `Error::RecordNotFound` for the create-returned-no-row case.
use crate::bulk_manifest::sql;
use crate::bulk_manifest::types::{
    BulkManifest, ConformanceField, CreateParams, STATUS_ACTIVE, STATUS_CLOSED, STATUS_EXPIRED,
};
use crate::error::{Error, Result};
use chrono::{Duration, Utc};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

/// Create active `bulk_manifest`. UNIQUE index on `sweep_id` refuses dupes.
///
/// # Errors
/// `Error::Surreal` from the CREATE query (including UNIQUE violation);
/// `Error::RecordNotFound` if CREATE returns zero rows.
pub async fn create(db: &Surreal<Db>, params: CreateParams<'_>) -> Result<BulkManifest> {
    let now = Utc::now();
    // `DateTime + Duration` is what clippy::arithmetic_side_effects flags;
    // `checked_add_signed` is the explicit-overflow form. Failure propagates
    // as `Error::Migration` since `ttl_seconds` is operator-controlled.
    let expires_at = now
        .checked_add_signed(Duration::seconds(params.ttl_seconds))
        .ok_or_else(|| {
            Error::Migration(format!(
                "ttl_seconds={} overflows DateTime range",
                params.ttl_seconds
            ))
        })?;
    let mut response = db
        .query(sql::SQL_CREATE)
        .bind(("sweep_id", params.sweep_id.to_owned()))
        .bind(("project", params.project.to_owned()))
        .bind(("root_rca", params.root_rca.to_owned()))
        .bind(("scope_glob", params.scope_glob.to_owned()))
        .bind(("lint_class", params.lint_class.to_owned()))
        .bind(("fix_strategy", params.fix_strategy.to_owned()))
        .bind(("blast_estimate", params.blast_estimate))
        .bind(("signed_by_session", params.signed_by_session.to_owned()))
        .bind(("approved_by", params.approved_by.to_owned()))
        .bind(("approved_at", now))
        .bind(("expires_at", expires_at))
        .bind(("status", STATUS_ACTIVE.to_owned()))
        .await?;
    response
        .take::<Option<BulkManifest>>(0)?
        .ok_or_else(|| Error::RecordNotFound("bulk_manifest create returned no row".to_owned()))
}

/// Fetch manifest by `sweep_id`. Ok(None) on miss.
///
/// # Errors
/// Propagates `Error::Surreal` from the SELECT query.
pub async fn get(db: &Surreal<Db>, sweep_id: &str) -> Result<Option<BulkManifest>> {
    let mut r = db
        .query(sql::SQL_GET)
        .bind(("sid", sweep_id.to_owned()))
        .await?;
    Ok(r.take(0)?)
}

/// Increment a conformance counter atomically. Column whitelist via enum.
///
/// # Errors
/// Propagates `Error::Surreal` from the UPDATE query.
pub async fn bump_conformance(
    db: &Surreal<Db>,
    sweep_id: &str,
    field: ConformanceField,
) -> Result<()> {
    let q = match field {
        ConformanceField::Applied => sql::SQL_BUMP_APPLIED,
        ConformanceField::Refused => sql::SQL_BUMP_REFUSED,
        ConformanceField::Drifted => sql::SQL_BUMP_DRIFTED,
    };
    db.query(q).bind(("sid", sweep_id.to_owned())).await?;
    Ok(())
}

/// Close manifest (agent finished). status=closed + `closed_at`=now.
///
/// # Errors
/// Propagates `Error::Surreal` from the UPDATE query.
pub async fn close(db: &Surreal<Db>, sweep_id: &str) -> Result<()> {
    set_terminal(db, sweep_id, STATUS_CLOSED).await
}

/// Mark expired (TTL fired). Distinct from close so audit preserves
/// "agent finished" vs "clock ran out".
///
/// # Errors
/// Propagates `Error::Surreal` from the UPDATE query.
pub async fn mark_expired(db: &Surreal<Db>, sweep_id: &str) -> Result<()> {
    set_terminal(db, sweep_id, STATUS_EXPIRED).await
}

async fn set_terminal(db: &Surreal<Db>, sid: &str, new_status: &str) -> Result<()> {
    db.query(sql::SQL_CLOSE)
        .bind(("st", new_status.to_owned()))
        .bind(("sid", sid.to_owned()))
        .bind(("active", STATUS_ACTIVE.to_owned()))
        .await?;
    Ok(())
}

/// List active manifests for a project. Used by `kavach bulk status` +
/// stop-gate (refuses clean stop while sweep is in-flight).
///
/// # Errors
/// Propagates `Error::Surreal` from the SELECT query.
pub async fn list_active(db: &Surreal<Db>, project: &str) -> Result<Vec<BulkManifest>> {
    let mut r = db
        .query(sql::SQL_LIST_ACTIVE)
        .bind(("proj", project.to_owned()))
        .bind(("active", STATUS_ACTIVE.to_owned()))
        .await?;
    Ok(r.take(0)?)
}
