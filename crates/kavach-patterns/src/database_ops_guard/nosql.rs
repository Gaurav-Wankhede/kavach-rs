//! NoSQL-specific database operation detection.

use super::pattern_set::hit;
use super::types::{DbOpsSeverity, DbOpsViolation, Store};

pub(super) fn detect(content: &str, violations: &mut Vec<DbOpsViolation>) {
    if hit(4, content) {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P0Block,
            store: Store::NoSql,
            pattern: "mongo-find-no-limit",
            fix: "find().toArray() without .limit() = OOM. Add .limit(N) + cursor pagination.",
            line: 0,
        });
    }
    if hit(5, content) {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P0Block,
            store: Store::NoSql,
            pattern: "mongo-where-injection",
            fix: "$where executes JS server-side. Use typed query operators instead.",
            line: 0,
        });
    }
    if hit(6, content) && !content.contains("writeConcern") && !content.contains("w:") {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P1Advisory,
            store: Store::NoSql,
            pattern: "mongo-write-no-concern",
            fix: "Specify writeConcern { w: 'majority' } for durability on critical writes.",
            line: 0,
        });
    }
}
