//! Core RCA gating: command-position matching, introspection short-circuits,
//! and the local-override env var.

use super::super::check_sqlx_migrate_requires_rca;

#[test]
fn blocks_sqlx_migrate_run_without_rca() {
    let r = check_sqlx_migrate_requires_rca("sqlx migrate run --source migrations", false);
    assert!(r.is_some(), "must block when RCA not satisfied");
    assert!(r.unwrap().contains("MIGRATE_RUN_REQUIRES_RCA"));
}

#[test]
fn allows_sqlx_migrate_run_with_rca() {
    let r = check_sqlx_migrate_requires_rca("sqlx migrate run --source migrations", true);
    assert!(r.is_none(), "RCA-satisfied turn must pass");
}

#[test]
fn allows_sqlx_migrate_info_without_rca() {
    let r = check_sqlx_migrate_requires_rca("sqlx migrate info --source migrations", false);
    assert!(r.is_none(), "sqlx migrate info is read-only");
}

#[test]
fn matches_cargo_sqlx_variant() {
    let r = check_sqlx_migrate_requires_rca("cargo sqlx migrate run", false);
    assert!(r.is_some(), "cargo sqlx variant must also be gated");
}

#[test]
fn skips_echo_with_phrase() {
    let r = check_sqlx_migrate_requires_rca("echo \"to apply: sqlx migrate run\"", false);
    assert!(
        r.is_none(),
        "echo of the phrase must not block; command position matters"
    );
}

#[test]
fn skips_comment_with_phrase() {
    let r = check_sqlx_migrate_requires_rca("# TODO: sqlx migrate run --source migrations", false);
    assert!(r.is_none(), "comment must not block");
}

#[test]
fn introspection_help_passes() {
    let r = check_sqlx_migrate_requires_rca("sqlx migrate run --help", false);
    assert!(r.is_none(), "--help must not trigger production-RCA gate");
}

#[test]
fn introspection_short_h_passes() {
    let r = check_sqlx_migrate_requires_rca("sqlx migrate run -h", false);
    assert!(r.is_none(), "-h must not trigger production-RCA gate");
}

#[test]
fn introspection_version_passes() {
    let r = check_sqlx_migrate_requires_rca("sqlx migrate run --version", false);
    assert!(
        r.is_none(),
        "--version must not trigger production-RCA gate"
    );
}

#[test]
fn dry_run_passes() {
    let r =
        check_sqlx_migrate_requires_rca("sqlx migrate run --source migrations --dry-run", false);
    assert!(
        r.is_none(),
        "--dry-run must not trigger production-RCA gate"
    );
}

#[test]
fn real_run_still_blocks_after_introspection_whitelist() {
    let r = check_sqlx_migrate_requires_rca("sqlx migrate run --source migrations", false);
    assert!(r.is_some(), "real migrate run must still block without RCA");
}

#[test]
fn local_db_override_bypasses() {
    temp_env::with_var("KAVACH_LOCAL_DB", Some("1"), || {
        let r = check_sqlx_migrate_requires_rca("sqlx migrate run", false);
        assert!(r.is_none(), "KAVACH_LOCAL_DB=1 must short-circuit");
    });
}
