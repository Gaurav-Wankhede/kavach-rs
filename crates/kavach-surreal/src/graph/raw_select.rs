// Ad-hoc READ entry point into the store (completes read CRUD alongside the
// typed listers): run an operator-supplied SurrealQL SELECT and return its rows
// as plain JSON. SELECT/INFO only — SurrealDB 3.x has no native readonly tx/user
// (surrealdb#1711), so the read-only guarantee is enforced here, statement by
// statement, after comment stripping. SOURCE: decision.raw-select-cli.
// https://surrealdb.com/docs/sdk/rust/methods/query
use crate::error::{Error, Result};
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;

/// Statement verbs this read path permits. Anything else (UPDATE/DELETE/CREATE/
/// REMOVE/DEFINE/INSERT/RELATE/BEGIN/…) is rejected before execution so a raw
/// query can never mutate or escape the single-writer routing.
const READ_VERBS: [&str; 2] = ["SELECT", "INFO"];

/// Run a read-only `SurrealQL` query and return all result sets as pretty JSON.
///
/// Every statement must begin with `SELECT` or `INFO` (checked after stripping
/// `--`/`#` line comments and `/* */` block comments); a write verb is refused
/// with `Error::Validation`. A missing `entity` table (brand-new graph) is the
/// empty case, returned as `[]` rather than an error.
///
/// # Errors
/// `Error::Validation` if the query is empty or contains a non-read statement;
/// `Error::Surreal` on a real query failure.
pub async fn raw_select(db: &Surreal<Db>, sql: &str) -> Result<String> {
    assert_read_only(sql)?;
    let mut resp = db.query(sql).await?;
    // Collect each statement's result set into one JSON array, bounded by the
    // KNOWN statement count (never probe open-endedly). take(i) on a generic
    // Value yields that statement's output; a missing-table error is the empty
    // graph, not a failure.
    let n = resp.num_statements();
    let mut sets: Vec<serde_json::Value> = Vec::with_capacity(n);
    for idx in 0..n {
        match resp.take::<surrealdb_types::Value>(idx) {
            Ok(v) => sets.push(v.into_json_value()),
            Err(e) if crate::error::is_missing_table_error(&e) => {
                sets.push(serde_json::Value::Array(Vec::new()));
            }
            Err(e) => return Err(e.into()),
        }
    }
    let body = if sets.len() == 1 {
        sets.into_iter().next().unwrap_or(serde_json::Value::Null)
    } else {
        serde_json::Value::Array(sets)
    };
    serde_json::to_string_pretty(&body).map_err(Error::Json)
}

/// Reject any statement that is not a SELECT/INFO. Splits on `;` after stripping
/// comments so `SELECT 1; DELETE entity` cannot smuggle a write past a prefix check.
fn assert_read_only(sql: &str) -> Result<()> {
    let stripped = strip_comments(sql);
    let mut saw_statement = false;
    for stmt in stripped.split(';') {
        let stmt = stmt.trim();
        if stmt.is_empty() {
            continue;
        }
        saw_statement = true;
        let verb = stmt.split_whitespace().next().unwrap_or("").to_uppercase();
        if !READ_VERBS.contains(&verb.as_str()) {
            return Err(Error::Validation(format!(
                "raw query is read-only: statement starting `{verb}` is not allowed \
                 (only SELECT/INFO). Use a typed verb (db write / mistake-purge / …) to mutate."
            )));
        }
    }
    if !saw_statement {
        return Err(Error::Validation("raw query is empty".to_owned()));
    }
    Ok(())
}

/// Strip `--` and `#` line comments and `/* */` block comments so the verb check
/// sees only executable text (a comment must never hide a write verb). Char-based
/// (no byte indexing) so multibyte content survives and no bound can panic.
fn strip_comments(sql: &str) -> String {
    let mut out = String::with_capacity(sql.len());
    let mut chars = sql.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '#' => skip_to_newline(&mut chars),
            '-' if chars.peek() == Some(&'-') => {
                chars.next();
                skip_to_newline(&mut chars);
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                skip_block(&mut chars);
            }
            other => out.push(other),
        }
    }
    out
}

/// Advance past the rest of the current line (the line comment's body).
fn skip_to_newline(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    for c in chars.by_ref() {
        if c == '\n' {
            break;
        }
    }
}

/// Advance past a `/* … */` block, consuming the closing `*/`.
fn skip_block(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    while let Some(c) = chars.next() {
        if c == '*' && chars.peek() == Some(&'/') {
            chars.next();
            break;
        }
    }
}

#[cfg(test)]
#[path = "raw_select_test.rs"]
mod raw_select_test;
