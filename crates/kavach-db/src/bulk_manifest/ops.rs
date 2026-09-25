// split: intentional - cohesive bulk_manifest CRUD surface (create/get/bump/close/list)
// Async DB ops for bulk_manifest. I/O surface only — types in `types.rs`,
// SQL in `sql.rs`. All params arrive via positional `?N` binds; no format!.
// Errors propagate via `?` through `Error::Turso` (#[from]) and
// `Error::RecordNotFound` for the create-returned-no-row case.
use crate::bulk_manifest::sql;
use crate::bulk_manifest::types::{
    BulkManifest, ConformanceField, CreateParams, STATUS_ACTIVE, STATUS_CLOSED, STATUS_EXPIRED,
};
use crate::error::{Error, Result};
use crate::turso::TursoStore;
use chrono::{DateTime, Duration, Utc};

/// Map a `bulk_manifest` row to its typed form. Column order MUST stay in sync
/// with `sql::MANIFEST_COLS`.
fn manifest_from_row(row: &libsql::Row) -> Result<BulkManifest> {
    let parse_dt = |i: i32| -> Result<DateTime<Utc>> {
        let s = row.get::<String>(i)?;
        DateTime::parse_from_rfc3339(&s)
            .map(|d| d.with_timezone(&Utc))
            .map_err(|e| Error::Migration(format!("bulk_manifest bad datetime col {i}: {e}")))
    };
    let id = row.get::<Option<i64>>(0)?;
    Ok(BulkManifest {
        id: id.map(|v| v.to_string()),
        sweep_id: row.get::<String>(1)?,
        project: row.get::<String>(2)?,
        root_rca: row.get::<String>(3)?,
        scope_glob: row.get::<String>(4)?,
        lint_class: row.get::<String>(5)?,
        fix_strategy: row.get::<String>(6)?,
        blast_estimate: row.get::<i64>(7)?,
        signed_by_session: row.get::<String>(8)?,
        approved_by: row.get::<String>(9)?,
        approved_at: parse_dt(10)?,
        expires_at: parse_dt(11)?,
        conformance_applied: row.get::<i64>(12)?,
        conformance_refused: row.get::<i64>(13)?,
        conformance_drifted: row.get::<i64>(14)?,
        status: row.get::<String>(15)?,
        closed_at: row
            .get::<Option<String>>(16)?
            .map(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .map(|d| d.with_timezone(&Utc))
                    .map_err(|e| Error::Migration(format!("bulk_manifest bad closed_at: {e}")))
            })
            .transpose()?,
    })
}

/// Create active `bulk_manifest`. UNIQUE index on `sweep_id` refuses dupes.
///
/// # Errors
/// `Error::Turso` from the INSERT query (including UNIQUE violation);
/// `Error::RecordNotFound` if INSERT returns zero rows.
pub async fn create(db: &TursoStore, params: CreateParams<'_>) -> Result<BulkManifest> {
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
        .exec_params(
            sql::SQL_CREATE,
            libsql::params![
                params.sweep_id.to_owned(),
                params.project.to_owned(),
                params.root_rca.to_owned(),
                params.scope_glob.to_owned(),
                params.lint_class.to_owned(),
                params.fix_strategy.to_owned(),
                params.blast_estimate,
                params.signed_by_session.to_owned(),
                params.approved_by.to_owned(),
                now.to_rfc3339(),
                expires_at.to_rfc3339(),
                STATUS_ACTIVE.to_owned()
            ],
        )
        .await?;
    response.next().await?.map_or_else(
        || {
            Err(Error::RecordNotFound(
                "bulk_manifest create returned no row".to_owned(),
            ))
        },
        |row| manifest_from_row(&row),
    )
}

/// Fetch manifest by `sweep_id`. Ok(None) on miss.
///
/// # Errors
/// Propagates `Error::Turso` from the SELECT query.
pub async fn get(db: &TursoStore, sweep_id: &str) -> Result<Option<BulkManifest>> {
    let mut r = db
        .exec_params(sql::SQL_GET, libsql::params![sweep_id.to_owned()])
        .await?;
    r.next()
        .await?
        .map_or_else(|| Ok(None), |row| manifest_from_row(&row).map(Some))
}

/// Increment a conformance counter atomically. Column whitelist via enum.
///
/// # Errors
/// Propagates `Error::Turso` from the UPDATE query.
pub async fn bump_conformance(
    db: &TursoStore,
    sweep_id: &str,
    field: ConformanceField,
) -> Result<()> {
    let q = match field {
        ConformanceField::Applied => sql::SQL_BUMP_APPLIED,
        ConformanceField::Refused => sql::SQL_BUMP_REFUSED,
        ConformanceField::Drifted => sql::SQL_BUMP_DRIFTED,
    };
    db.exec_params(q, libsql::params![sweep_id.to_owned()])
        .await?;
    Ok(())
}

/// Close manifest (agent finished). status=closed + `closed_at`=now.
///
/// # Errors
/// Propagates `Error::Turso` from the UPDATE query.
pub async fn close(db: &TursoStore, sweep_id: &str) -> Result<()> {
    set_terminal(db, sweep_id, STATUS_CLOSED).await
}

/// Mark expired (TTL fired). Distinct from close so audit preserves
/// "agent finished" vs "clock ran out".
///
/// # Errors
/// Propagates `Error::Turso` from the UPDATE query.
pub async fn mark_expired(db: &TursoStore, sweep_id: &str) -> Result<()> {
    set_terminal(db, sweep_id, STATUS_EXPIRED).await
}

async fn set_terminal(db: &TursoStore, sid: &str, new_status: &str) -> Result<()> {
    db.exec_params(
        sql::SQL_CLOSE,
        libsql::params![
            new_status.to_owned(),
            Utc::now().to_rfc3339(),
            sid.to_owned(),
            STATUS_ACTIVE.to_owned()
        ],
    )
    .await?;
    Ok(())
}

/// List active manifests for a project. Used by `kavach bulk status` +
/// stop-gate (refuses clean stop while sweep is in-flight).
///
/// # Errors
/// Propagates `Error::Turso` from the SELECT query.
pub async fn list_active(db: &TursoStore, project: &str) -> Result<Vec<BulkManifest>> {
    let mut r = db
        .exec_params(
            sql::SQL_LIST_ACTIVE,
            libsql::params![project.to_owned(), STATUS_ACTIVE.to_owned()],
        )
        .await?;
    let mut out = Vec::new();
    while let Some(row) = r.next().await? {
        out.push(manifest_from_row(&row)?);
    }
    Ok(out)
}
