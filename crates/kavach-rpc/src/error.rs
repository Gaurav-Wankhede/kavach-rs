use jsonrpsee::types::ErrorObjectOwned;
use kavach_db::Error as DbError;

pub const CODE_INVALID_PARAMS: i32 = -32602;
pub const CODE_INTERNAL: i32 = -32603;
pub const CODE_DB_PROJECT_NOT_FOUND: i32 = -32001;
pub const CODE_DB_RECORD_NOT_FOUND: i32 = -32002;
pub const CODE_DB_MIGRATION: i32 = -32003;
pub const CODE_DB_IO: i32 = -32004;
pub const CODE_DB_JSON: i32 = -32005;
pub const CODE_DB_INVALID_HIERARCHY: i32 = -32007;
pub const CODE_DB_SCHEMA_VIOLATION: i32 = -32008;
pub const CODE_DB_VALIDATION: i32 = -32009;
pub const CODE_DB_TURSO: i32 = -32010;

#[must_use]
#[expect(
    clippy::needless_pass_by_value,
    reason = "consumed at .map_err(db_to_rpc) call sites across 68 handlers; by-value avoids a borrow at every caller"
)]
pub fn db_to_rpc(err: DbError) -> ErrorObjectOwned {
    let (code, msg) = match &err {
        DbError::ProjectNotFound(p) => {
            (CODE_DB_PROJECT_NOT_FOUND, format!("project not found: {p}"))
        }
        DbError::RecordNotFound(r) => {
            (CODE_DB_RECORD_NOT_FOUND, format!("record not found: {r}"))
        }
        DbError::Migration(m) => (CODE_DB_MIGRATION, format!("migration error: {m}")),
        DbError::Io(e) => (CODE_DB_IO, format!("io error: {e}")),
        DbError::Json(e) => (CODE_DB_JSON, format!("json error: {e}")),
        DbError::InvalidHierarchy(h) => {
            (CODE_DB_INVALID_HIERARCHY, format!("invalid hierarchy: {h}"))
        }
        DbError::SchemaViolation(v) => {
            (CODE_DB_SCHEMA_VIOLATION, format!("schema violation: {v}"))
        }
        DbError::Validation(v) => (CODE_DB_VALIDATION, format!("validation error: {v}")),
        DbError::Db(e) => (CODE_DB_TURSO, format!("turso error: {e}")),
        DbError::Libsql(e) => (CODE_DB_TURSO, format!("libsql error: {e}")),
    };
    ErrorObjectOwned::owned(code, msg, None::<()>)
}

pub fn invalid_params(msg: impl Into<String>) -> ErrorObjectOwned {
    ErrorObjectOwned::owned(CODE_INVALID_PARAMS, msg.into(), None::<()>)
}

pub fn internal(msg: impl Into<String>) -> ErrorObjectOwned {
    ErrorObjectOwned::owned(CODE_INTERNAL, msg.into(), None::<()>)
}
