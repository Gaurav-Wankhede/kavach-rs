//! HARD BLOCK for `sqlx migrate run` against a production DB without an [RCA]
//! block this turn. Local/dev databases (localhost, 127.0.0.1, docker-compose,
//! Unix sockets) are auto-detected from `DATABASE_URL` and exempted.
use super::segment::segment_first_word_is;
#[cfg(test)]
#[path = "sqlx_migrate_test.rs"]
mod tests;
/// Detect if `DATABASE_URL` points to a local/dev database. Local patterns:
/// localhost, 127.0.0.1, `::1`, docker-compose service names, Unix sockets.
/// RESEARCH: github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md
fn is_local_database_url() -> bool {
    let url = match std::env::var("DATABASE_URL") {
        Ok(u) => u.to_lowercase(),
        Err(_) => return false,
    };
    let local_hosts = [
        "@localhost",
        "@127.0.0.1",
        "@[::1]",
        "@::1",
        ".local/",
        ".local:",
        "@host.docker.internal",
        "@postgres:",
        "@postgres/",
        "@db:",
        "@db/",
        "@database:",
        "@database/",
        "@postgresql:",
        "@postgresql/",
    ];
    for host in local_hosts {
        if url.contains(host) {
            return true;
        }
    }
    // Unix socket paths are local.
    if url.contains("unix:") || url.contains("/var/run/") || url.contains("/tmp/") {
        return true;
    }
    false
}
/// `Some(reason)` when the command is `sqlx migrate run ...` (sqlx or cargo-sqlx
/// in command position) AND no RCA was recorded this turn AND the target is not
/// a local/dev DB. Read-only introspection (`--help`/`-h`/`--version`/
/// `--dry-run`) and the `KAVACH_LOCAL_DB=1` override short-circuit.
/// SOURCE: github.com/launchbadge/sqlx/discussions/1292 — applied migrations
/// cause irreversible checksum drift; treat as a production write requiring RCA.
pub(in crate::gates::pre_tool_bash) fn check_sqlx_migrate_requires_rca(
    cmd: &str,
    rca_satisfied: bool,
) -> Option<String> {
    let lower = cmd.trim().to_lowercase();
    let starts_with_cmd =
        segment_first_word_is(&lower, "sqlx") || segment_first_word_is(&lower, "cargo");
    if !starts_with_cmd {
        return None;
    }
    let is_migrate_run =
        lower.contains("sqlx migrate run") || lower.contains("cargo sqlx migrate run");
    if !is_migrate_run {
        return None;
    }
    let is_introspection = lower.contains(" --help")
        || lower.ends_with(" -h")
        || lower.contains(" -h ")
        || lower.contains(" --version")
        || lower.ends_with(" --version")
        || lower.contains(" --dry-run");
    if is_introspection {
        return None;
    }
    if std::env::var("KAVACH_LOCAL_DB").is_ok_and(|v| v == "1") {
        return None;
    }
    if is_local_database_url() {
        return None;
    }
    if rca_satisfied {
        return None;
    }
    Some(
        "MIGRATE_RUN_REQUIRES_RCA: `sqlx migrate run` mutates shared DB state. \
         CLAUDE.md requires an [RCA] block in this turn before any destructive \
         action against production. Output [RCA] then retry, OR set \
         KAVACH_LOCAL_DB=1 if this is a local/test DB. \
         SOURCE: github.com/launchbadge/sqlx/discussions/1292"
            .to_owned(),
    )
}
