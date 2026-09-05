// Compile-time SQL constants for bulk_manifest ops. ALL queries are
// `concat!()` strings; parameters arrive ONLY via positional `?N` binds.
// Column whitelist for conformance bumps switches between three constants
// via a closed Rust enum — no interpolation, no user-derived column names.

// Column order shared by every SELECT/RETURNING below — keep in sync with
// `ops::manifest_from_row` indices:
// id, sweep_id, project, root_rca, scope_glob, lint_class, fix_strategy,
// blast_estimate, signed_by_session, approved_by, approved_at, expires_at,
// conformance_applied, conformance_refused, conformance_drifted, status, closed_at

pub(super) const SQL_CREATE: &str = concat!(
    "INSERT INTO bulk_manifest (sweep_id, project, root_rca, scope_glob, lint_class, ",
    "fix_strategy, blast_estimate, signed_by_session, approved_by, ",
    "approved_at, expires_at, status) ",
    "VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12) ",
    "RETURNING ",
    "id, sweep_id, project, root_rca, scope_glob, lint_class, ",
    "fix_strategy, blast_estimate, signed_by_session, approved_by, ",
    "approved_at, expires_at, conformance_applied, conformance_refused, ",
    "conformance_drifted, status, closed_at",
);

pub(super) const SQL_GET: &str = concat!(
    "SELECT ",
    "id, sweep_id, project, root_rca, scope_glob, lint_class, ",
    "fix_strategy, blast_estimate, signed_by_session, approved_by, ",
    "approved_at, expires_at, conformance_applied, conformance_refused, ",
    "conformance_drifted, status, closed_at",
    " FROM bulk_manifest WHERE sweep_id = ?1 LIMIT 1",
);

pub(super) const SQL_LIST_ACTIVE: &str = concat!(
    "SELECT ",
    "id, sweep_id, project, root_rca, scope_glob, lint_class, ",
    "fix_strategy, blast_estimate, signed_by_session, approved_by, ",
    "approved_at, expires_at, conformance_applied, conformance_refused, ",
    "conformance_drifted, status, closed_at",
    " FROM bulk_manifest WHERE project = ?1 AND status = ?2 ",
    "ORDER BY approved_at DESC LIMIT 50",
);

pub(super) const SQL_BUMP_APPLIED: &str = "UPDATE bulk_manifest SET conformance_applied = conformance_applied + 1 \
     WHERE sweep_id = ?1";

pub(super) const SQL_BUMP_REFUSED: &str = "UPDATE bulk_manifest SET conformance_refused = conformance_refused + 1 \
     WHERE sweep_id = ?1";

pub(super) const SQL_BUMP_DRIFTED: &str = "UPDATE bulk_manifest SET conformance_drifted = conformance_drifted + 1 \
     WHERE sweep_id = ?1";

pub(super) const SQL_CLOSE: &str = "UPDATE bulk_manifest SET status = ?1, closed_at = ?2 \
     WHERE sweep_id = ?3 AND status = ?4";
