use crate::error::{Error, Result};
use crate::turso::TursoStore;

/// Statement verbs this read path permits. Anything else (UPDATE/DELETE/CREATE/
/// REMOVE/DEFINE/INSERT/RELATE/BEGIN/…) is rejected before execution so a raw
/// query can never mutate or escape the single-writer routing.
const READ_VERBS: [&str; 2] = ["SELECT", "INFO"];

/// Run a read-only SQL query and return all rows as pretty JSON.
///
/// Every statement must begin with `SELECT` or `INFO` (checked after stripping
/// `--`/`#` line comments and `/* */` block comments); a write verb is refused
/// with `Error::Validation`. A missing table (brand-new graph) is the empty
/// case, returned as `[]` rather than an error.
///
/// # Errors
/// `Error::Validation` if the query is empty or contains a non-read statement;
/// `Error::Turso` on a real query failure.
pub async fn raw_select(db: &TursoStore, sql: &str) -> Result<String> {
    assert_read_only(sql)?;
    let mut resp = match db.exec_params(sql, ()).await {
        Ok(resp) => resp,
        // A missing table is the brand-new graph, not a failure: return `[]`.
        Err(crate::turso::TursoError::Libsql(libsql::Error::SqliteFailure(_, msg)))
            if msg.contains("no such table") =>
        {
            return Ok("[]".to_owned());
        }
        Err(e) => return Err(Error::Turso(e)),
    };
    // Collect every row into one JSON array.
    let mut rows_out: Vec<serde_json::Value> = Vec::new();
    while let Some(row) = resp.next().await? {
        let mut obj = serde_json::Map::new();
        let ncols = row.column_count();
        for i in 0..ncols {
            let col = row.column_name(i).unwrap_or_default().to_owned();
            let v = match row.get::<Option<String>>(i) {
                Ok(Some(s)) => serde_json::Value::String(s),
                Ok(None) => serde_json::Value::Null,
                Err(_) => match row.get::<Option<i64>>(i) {
                    Ok(Some(n)) => serde_json::Value::Number(n.into()),
                    Ok(None) => serde_json::Value::Null,
                    Err(_) => match row.get::<Option<f64>>(i) {
                        Ok(Some(f)) => serde_json::Number::from_f64(f)
                            .map(serde_json::Value::Number)
                            .unwrap_or(serde_json::Value::Null),
                        Ok(None) => serde_json::Value::Null,
                        Err(_) => serde_json::Value::Null,
                    },
                },
            };
            obj.insert(col, v);
        }
        rows_out.push(serde_json::Value::Object(obj));
    }
    let body = if rows_out.len() == 1 {
        rows_out
            .into_iter()
            .next()
            .unwrap_or(serde_json::Value::Null)
    } else {
        serde_json::Value::Array(rows_out)
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
