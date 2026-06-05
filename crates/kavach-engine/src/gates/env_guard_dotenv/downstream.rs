//! Binary allowlist for the command following `source .env` — safe iff it consumes
//! env vars without echoing them (fail-closed: unknown binaries are rejected).

/// Return true when the post-source command is one that consumes env vars without
/// echoing them — e.g. sqlx migrate, cargo run, npm run.
///
/// Python is BANNED per ~/.claude/rules/04-anti-patterns.md global Python ban.
/// psql is BANNED — canonical escape hatch for bypassing sqlx checksum validation.
/// `kavach` is allowed so `source .env && kavach db pg-fix-checksum ...` works —
/// kavach sub-commands take DSN via --dsn flag and never print env values.
pub(crate) fn is_safe_downstream(downstream: &str) -> bool {
    let lc = downstream.trim().to_lowercase();
    if lc.starts_with("database_url=") || lc.starts_with("cd ") {
        return true;
    }
    let Some(first_token) = lc.split_whitespace().next() else {
        return false;
    };
    let basename = std::path::Path::new(first_token)
        .file_name()
        .and_then(|n| n.to_str())
        .map_or(first_token, |b| b);
    let safe_binaries = [
        "sqlx",
        "cargo",
        "bun",
        "make",
        "npm",
        "pnpm",
        "node",
        "deno",
        "go",
        "diesel",
        "flyway",
        "liquibase",
        "alembic",
        "migrate",
        "kavach",
    ];
    safe_binaries.contains(&basename)
}
