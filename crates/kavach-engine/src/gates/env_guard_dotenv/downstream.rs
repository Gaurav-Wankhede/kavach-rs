//! Binary allowlist for the command following `source .env` — safe iff it consumes
//! env vars without echoing them (fail-closed: unknown binaries are rejected).

/// Return true when the post-source command is one that consumes env vars without
/// echoing them — e.g. sqlx migrate, cargo run, npm run, or a non-destructive psql.
///
/// Python is BANNED per ~/.claude/rules/04-anti-patterns.md global Python ban.
/// `psql` is allowed for READ/INSERT/UPDATE/CREATE — but a destructive verb
/// (DELETE/DROP/TRUNCATE) anywhere in the command makes it unsafe here, and the
/// dedicated psql write-bypass gate hard-blocks it regardless. This is the
/// defense-in-depth first line so the env-leak gate doesn't wave it through.
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
    // psql is conditionally safe: allowed only when it carries no destructive
    // SQL verb. Shared classifier keeps identifier substrings (deleted_at) safe.
    // Recognise psql as the leading binary OR anywhere in a compound downstream
    // (`echo ..; psql ..`, `psql .. | head`) — a harmless prefix/pipe must not mask
    // a safe psql. The destructive-verb classifier stays the real safety gate.
    if basename == "psql" || invokes_psql(&lc) {
        return crate::gates::sql_destructive::destructive_sql_keyword(&lc).is_none();
    }
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

/// True when a `psql` command appears at any command boundary in a compound
/// downstream (`echo ..; psql ..`, `psql .. | head`, `a && psql ..`), so a
/// harmless prefix or pipe does not mask a safe psql. Matches `psql` only as a
/// command word (boundary-prefixed + word-terminated), never the substring of
/// another token. The destructive-SQL classifier remains the real safety gate.
fn invokes_psql(lc: &str) -> bool {
    lc.split([';', '|', '&'])
        .map(str::trim)
        .filter_map(|seg| seg.split_whitespace().next())
        .any(|tok| {
            std::path::Path::new(tok)
                .file_name()
                .and_then(|n| n.to_str())
                == Some("psql")
        })
}
