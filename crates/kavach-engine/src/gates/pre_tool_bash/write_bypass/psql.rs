//! HARD BLOCK for bare `psql` in command position. Only `sqlx migrate` is the
//! sanctioned DB path; `psql` bypasses the migration pipeline (untracked schema
//! changes). Quote-aware: a `psql` token inside a quoted arg is inert data.

use super::segment::segment_first_word_is;
use crate::gates::pre_tool_bash::strip_quoted_regions;

// FIX: [CWE-184 over-broad-trigger] a psql token inside another tool's quoted
// arg (`rg -n 'a|psql' s`, `echo 'x | psql y'`) HARD-BLOCKED as pipeline-to-psql.
// ROOT_CAUSE: lexical/separator detection without quote-state.
// SOLUTION: strip quoted spans to inert placeholders via the shared
// strip_quoted_regions primitive BEFORE the command-position check.
// RESEARCH: https://cwe.mitre.org/data/definitions/184.html

/// `Some(reason)` when `psql` is in command position (after quote stripping).
/// sqlx CLI has no `exec` (issue #1375); for ad-hoc queries use `sqlx::query!`
/// in a Rust integration test instead.
pub(in crate::gates::pre_tool_bash) fn check_psql_blocked(cmd: &str) -> Option<String> {
    let stripped = strip_quoted_regions(cmd.trim()).to_lowercase();
    if !segment_first_word_is(&stripped, "psql") {
        return None;
    }
    Some(
        "PSQL_BLOCKED: `psql` is banned. Use `sqlx` for all database operations.\n\
         Schema changes: `sqlx migrate run --source <dir>`\n\
         Check status: `sqlx migrate info --source <dir>`\n\
         Verify schema: write a Rust integration test with `sqlx::query!`\n\
         Ad-hoc queries: use `sqlx::query_as` in a test or one-off binary."
            .to_owned(),
    )
}
