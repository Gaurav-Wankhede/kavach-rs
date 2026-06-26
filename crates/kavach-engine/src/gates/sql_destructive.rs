//! Single source of truth for "is this SQL command destructive?". Shared by the
//! psql write-bypass gate and the dotenv env-leak gate so both layers agree on
//! exactly which verbs are irreversible. The safety boundary is the SQL
//! operation: DELETE / DROP / TRUNCATE are irreversible (hard-blocked); SELECT /
//! INSERT / UPDATE / CREATE are not.

/// Irreversible SQL verbs. `DROP DATABASE` is covered by the `drop` prefix.
const DESTRUCTIVE_KEYWORDS: &[&str] = &["delete", "drop", "truncate"];

/// `Some(keyword)` when `cmd` carries a destructive SQL verb as a standalone
/// token. Word-boundary aware (on `_`/alphanumeric) so identifiers that merely
/// contain a keyword — `deleted_at`, `dropdown_options`, `truncate_log` — do NOT
/// trigger. Scans the raw command lowercased; callers pass the original text
/// (NOT a quote-stripped copy) since the SQL payload lives inside quoted args.
pub(crate) fn destructive_sql_keyword(cmd: &str) -> Option<&'static str> {
    let lc = cmd.to_lowercase();
    DESTRUCTIVE_KEYWORDS
        .iter()
        .copied()
        .find(|&kw| has_sql_keyword(&lc, kw))
}

/// True when `kw` appears in the already-lowercased `haystack` delimited by a
/// non-identifier byte on both sides — i.e. as an SQL token, not a substring.
/// Byte-based (kw is ASCII): identifier = ASCII alphanumeric or `_`.
fn has_sql_keyword(haystack: &str, kw: &str) -> bool {
    let hay = haystack.as_bytes();
    let needle = kw.as_bytes();
    if needle.is_empty() || needle.len() > hay.len() {
        return false;
    }
    let is_ident = |b: &u8| b.is_ascii_alphanumeric() || *b == b'_';
    hay.windows(needle.len()).enumerate().any(|(start, win)| {
        if win != needle {
            return false;
        }
        let prev_ok = start
            .checked_sub(1)
            .and_then(|p| hay.get(p))
            .is_none_or(|b| !is_ident(b));
        let next_ok = hay
            .get(start.saturating_add(needle.len()))
            .is_none_or(|b| !is_ident(b));
        prev_ok && next_ok
    })
}

/// The P0 reason string for a destructive SQL operation. Shared so every entry
/// point (psql, sourced runner) emits the same hard-block message.
pub(crate) fn destructive_sql_reason(keyword: &str) -> String {
    format!(
        "SQL_DELETE_BLOCKED: `{}` is an irreversible DB operation and is hard-blocked (P0). \
         READ (SELECT), INSERT, UPDATE, and CREATE are allowed; DELETE / DROP / TRUNCATE are not. \
         For an intended removal, write a reviewed `sqlx migrate` step so it is tracked and reversible.",
        keyword.to_uppercase()
    )
}

#[cfg(test)]
mod tests {
    use super::destructive_sql_keyword;

    #[test]
    fn allows_read_and_write_ops() {
        assert!(destructive_sql_keyword("psql $DSN -c 'SELECT * FROM users'").is_none());
        assert!(destructive_sql_keyword("psql $DSN -c 'INSERT INTO t VALUES (1)'").is_none());
        assert!(destructive_sql_keyword("psql $DSN -c 'UPDATE t SET x=1 WHERE id=2'").is_none());
        assert!(destructive_sql_keyword("psql $DSN -c 'CREATE TABLE t (id int)'").is_none());
    }

    #[test]
    fn blocks_destructive_ops() {
        assert_eq!(
            destructive_sql_keyword("psql $DSN -c 'DELETE FROM users WHERE id=1'"),
            Some("delete")
        );
        assert_eq!(
            destructive_sql_keyword("psql $DSN -c 'DROP TABLE users'"),
            Some("drop")
        );
        assert_eq!(
            destructive_sql_keyword("psql $DSN -c 'TRUNCATE audit_log'"),
            Some("truncate")
        );
        assert_eq!(
            destructive_sql_keyword("psql $DSN -c 'DROP DATABASE prod'"),
            Some("drop")
        );
    }

    #[test]
    fn identifier_substrings_do_not_trigger() {
        assert!(destructive_sql_keyword("psql $DSN -c 'SELECT deleted_at FROM users'").is_none());
        assert!(destructive_sql_keyword("psql $DSN -c 'SELECT * FROM dropdown_options'").is_none());
        assert!(
            destructive_sql_keyword("psql $DSN -c 'SELECT truncate_log FROM settings'").is_none()
        );
    }
}
