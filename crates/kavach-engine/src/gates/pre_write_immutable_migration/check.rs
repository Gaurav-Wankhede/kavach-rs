//! Gate entry point: block edits to applied migrations unless overridden.
use super::applied::{APPLIED_HEURISTIC_DAYS, is_presumed_applied};
use super::classify::is_migration_file;
use std::path::Path;

const OVERRIDE_ENV: &str = "KAVACH_ALLOW_MIGRATION_EDIT";

/// `Some(reason)` when editing a presumed-applied sqlx migration without override.
pub(crate) fn check(target_path: &str) -> Option<String> {
    if !is_migration_file(target_path) {
        return None;
    }
    if std::env::var(OVERRIDE_ENV).is_ok_and(|v| v == "1") {
        return None;
    }
    if !is_presumed_applied(Path::new(target_path)) {
        return None;
    }
    Some(format!(
        "IMMUTABLE_MIGRATION: {target_path} appears to be an applied sqlx migration \
         (last commit >={APPLIED_HEURISTIC_DAYS}d ago). Editing it breaks the sqlx \
         checksum ledger (sqlx Discussion #1292). \n\
         If reconciling drift: set {OVERRIDE_ENV}=1 and back up _sqlx_migrations first. \n\
         If this is a fresh migration mistakenly looking old: touch the file or \
         create a new migration with a higher version number. \n\
         SOURCE: https://github.com/launchbadge/sqlx/discussions/1292 + decision:rca.immutable_migration_gate"
    ))
}
