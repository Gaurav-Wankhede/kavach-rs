//! Detection-view selection: DB-client/shell payloads stay verbatim; other
//! tools' quoted args are stripped (CWE-184 false-positive exemption).
//!
//! §RADIUS-INTEGRITY (hunt.cwe184-prod-guard-command-class-dispatch):
//! a DB client like `psql -c 'DROP DATABASE foo'` legitimately delivers the
//! destructive payload as a quoted arg (the Pocket OS attack vector); stripping
//! quotes there would blank the very thing this guard exists to catch.
//! Conversely, `git commit -m "fix: DROP DATABASE"` merely MENTIONS the verb as
//! data — stripping quotes correctly exempts it.
use crate::gates::pre_tool_bash::strip_quoted_regions;

/// True iff the command-position word in any pipeline segment is a known
/// database client or shell whose `-c`/`-e`/`--eval` flag delivers SQL/commands
/// as a quoted argument. Conservative allowlist — keeps quote-strip OFF for any
/// tool that interprets quoted args as commands.
fn is_db_client_command(lower: &str) -> bool {
    // PRESERVE-PAYLOAD allowlist: tools whose `-c`/`-e` flag delivers a payload
    // that MUST be inspected verbatim. Includes DB clients AND shells (a shell's
    // `-c "DROP DATABASE foo"` executes the payload exactly like a DB client's
    // `-c`; treating them identically is the only safe routing).
    const DB_CLIENTS: &[&str] = &[
        "psql",
        "mysql",
        "mariadb",
        "mongo",
        "mongosh",
        "cqlsh",
        "redis-cli",
        "valkey-cli",
        "clickhouse-client",
        "sqlite3",
        "cockroach",
        "duckdb",
        "surreal",
        "sqlcmd", // DB clients
        "bash",
        "sh",
        "zsh",
        "fish",
        "dash",
        "ksh",
        "ash", // shells with -c payload
    ];
    // A tool's command word is the first whitespace token of any segment
    // delimited by &&/||/;/|/&. Matches legacy_tool_guard command-position.
    for segment in lower.split(['&', '|', ';', '(', '{', '\n']) {
        if let Some(word) = segment.split_whitespace().next() {
            // Strip a leading path: /usr/bin/psql → psql.
            let base = word.rsplit('/').next().unwrap_or(word);
            if DB_CLIENTS.contains(&base) {
                return true;
            }
        }
    }
    false
}

/// Detection view: the original (lowercased) command when a DB client is
/// involved (preserve -c payload for Pocket-OS-class detection), otherwise the
/// quote-stripped lowercased view (exempt quoted-arg literals in non-DB tools).
pub(super) fn detection_view(command: &str) -> String {
    let lower = command.to_lowercase();
    if is_db_client_command(&lower) {
        lower
    } else {
        strip_quoted_regions(command).to_lowercase()
    }
}
