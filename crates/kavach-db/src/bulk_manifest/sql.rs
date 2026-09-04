// Compile-time SQL constants for bulk_manifest ops. ALL queries are
// `concat!()` strings; parameters arrive ONLY via `bind(("name", val))`.
// Column whitelist for conformance bumps switches between three constants
// via a closed Rust enum — no interpolation, no user-derived column names.

pub(super) const SQL_CREATE: &str = concat!(
    "CREATE bulk_manifest CONTENT { ",
    "sweep_id: $sweep_id, project: $project, root_rca: $root_rca, ",
    "scope_glob: $scope_glob, lint_class: $lint_class, ",
    "fix_strategy: $fix_strategy, blast_estimate: $blast_estimate, ",
    "signed_by_session: $signed_by_session, approved_by: $approved_by, ",
    "approved_at: $approved_at, expires_at: $expires_at, ",
    "conformance_applied: 0, conformance_refused: 0, ",
    "conformance_drifted: 0, status: $status ",
    "} RETURN id, sweep_id, project, root_rca, scope_glob, lint_class, ",
    "fix_strategy, blast_estimate, signed_by_session, approved_by, ",
    "approved_at, expires_at, conformance_applied, conformance_refused, ",
    "conformance_drifted, status, closed_at",
);

pub(super) const SQL_GET: &str = concat!(
    "SELECT id, sweep_id, project, root_rca, scope_glob, lint_class, ",
    "fix_strategy, blast_estimate, signed_by_session, approved_by, ",
    "approved_at, expires_at, conformance_applied, conformance_refused, ",
    "conformance_drifted, status, closed_at ",
    "FROM bulk_manifest WHERE sweep_id = $sid LIMIT 1",
);

pub(super) const SQL_LIST_ACTIVE: &str = concat!(
    "SELECT id, sweep_id, project, root_rca, scope_glob, lint_class, ",
    "fix_strategy, blast_estimate, signed_by_session, approved_by, ",
    "approved_at, expires_at, conformance_applied, conformance_refused, ",
    "conformance_drifted, status, closed_at ",
    "FROM bulk_manifest WHERE project = $proj AND status = $active ",
    "ORDER BY approved_at DESC LIMIT 50",
);

pub(super) const SQL_BUMP_APPLIED: &str = "UPDATE bulk_manifest SET conformance_applied = conformance_applied + 1 \
     WHERE sweep_id = $sid";

pub(super) const SQL_BUMP_REFUSED: &str = "UPDATE bulk_manifest SET conformance_refused = conformance_refused + 1 \
     WHERE sweep_id = $sid";

pub(super) const SQL_BUMP_DRIFTED: &str = "UPDATE bulk_manifest SET conformance_drifted = conformance_drifted + 1 \
     WHERE sweep_id = $sid";

pub(super) const SQL_CLOSE: &str = "UPDATE bulk_manifest SET status = $st, closed_at = time::now() \
     WHERE sweep_id = $sid AND status = $active";
