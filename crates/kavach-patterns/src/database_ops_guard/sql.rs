//! SQL-specific database operation detection.

use super::pattern_set::{DESTRUCTIVE_SQL, FORMAT_SQL, SELECT_STAR, hit, matches_of};
use super::types::{DbOpsSeverity, DbOpsViolation, Store};

pub(super) fn detect(content: &str, violations: &mut Vec<DbOpsViolation>) {
    if SELECT_STAR.as_ref().is_some_and(|re| re.is_match(content)) {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P1Advisory,
            store: Store::Sql,
            pattern: "select-star",
            fix: "Name columns explicitly. Schema drift breaks consumers.",
            line: 0,
        });
    }
    if FORMAT_SQL.as_ref().is_some_and(|re| re.is_match(content)) {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P0Block,
            store: Store::Sql,
            pattern: "format-string-sql",
            fix: "SQL injection. Use sqlx::query!() with $1 params.",
            line: 0,
        });
    }
    if DESTRUCTIVE_SQL
        .as_ref()
        .is_some_and(|re| re.is_match(content))
    {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P0Block,
            store: Store::Sql,
            pattern: "destructive-sql",
            fix: "Hard delete/drop on entity table. Use soft-delete or migration with backup.",
            line: 0,
        });
    }
    if hit(1, content) && !content.contains("FOR UPDATE") {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P1Advisory,
            store: Store::Sql,
            pattern: "update-without-for-update",
            fix: "Multi-row UPDATE under READ COMMITTED risks lost updates. Add SELECT ... FOR UPDATE.",
            line: 0,
        });
    }
    let set_without_local = matches_of(2, content)
        .into_iter()
        .any(|m| !m.as_str().to_ascii_uppercase().contains("SET LOCAL"));
    if set_without_local {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P0Block,
            store: Store::Sql,
            pattern: "set-without-local",
            fix: "SET without LOCAL leaks across pooled connections. Use SET LOCAL inside transaction.",
            line: 0,
        });
    }
    if hit(3, content) {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P1Advisory,
            store: Store::Sql,
            pattern: "offset-pagination",
            fix: "OFFSET is O(n) on large tables. Use keyset: WHERE id > $last_id ORDER BY id LIMIT n.",
            line: 0,
        });
    }
    if hit(14, content) && !content.contains("timeout") {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P2Warning,
            store: Store::Sql,
            pattern: "pool-acquire-no-timeout",
            fix: "Wrap pool.acquire() in tokio::time::timeout to prevent worker stall under contention.",
            line: 0,
        });
    }
    if hit(15, content) {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P0Block,
            store: Store::Sql,
            pattern: "cassandra-allow-filtering",
            fix: "ALLOW FILTERING = full partition scan. Add secondary index or redesign partition key.",
            line: 0,
        });
    }
}
