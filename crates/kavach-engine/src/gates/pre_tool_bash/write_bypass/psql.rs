//! Operation-aware `psql` gate. `psql` in command position is ALLOWED for
//! READ (SELECT) / INSERT / UPDATE / CREATE, and HARD-BLOCKED (P0) only when it
//! carries an irreversible verb — DELETE / DROP / TRUNCATE. The safety boundary
//! is the SQL operation, not the binary. Quote-aware: a `psql` (or keyword)
//! token inside another tool's quoted arg is inert data.

use super::segment::segment_first_word_is;
use crate::gates::pre_tool_bash::strip_quoted_regions;
use crate::gates::sql_destructive::{destructive_sql_keyword, destructive_sql_reason};

// psql gate: block DELETE/DROP/TRUNCATE, allow SELECT/INSERT/UPDATE/CREATE.
// See decision.engine.psql_destructive_gate.

/// `Some(reason)` only when `psql` is in command position AND the command
/// carries a destructive SQL verb (DELETE/DROP/TRUNCATE). Non-destructive psql
/// (SELECT/INSERT/UPDATE/CREATE) returns `None` — allowed.
pub(in crate::gates::pre_tool_bash) fn check_psql_blocked(cmd: &str) -> Option<String> {
    let stripped = strip_quoted_regions(cmd.trim()).to_lowercase();
    if !segment_first_word_is(&stripped, "psql") {
        return None;
    }
    // Destructive check runs on the ORIGINAL command: the SQL text lives inside
    // the `-c '...'` quoted arg, which quote-stripping would erase. The shared
    // classifier scans raw with word-boundary matching, so identifier substrings
    // (deleted_at, dropdown) stay safe.
    destructive_sql_keyword(cmd).map(destructive_sql_reason)
}
