//! Key-Value store-specific database operation detection.

use super::pattern_set::hit;
use super::types::{DbOpsSeverity, DbOpsViolation, Store};

pub(super) fn detect(content: &str, violations: &mut Vec<DbOpsViolation>) {
    if content.contains("redis::")
        && hit(7, content)
        && !content.contains("SETEX")
        && !content.contains("EX ")
        && !content.contains("expire")
    {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P1Advisory,
            store: Store::Kv,
            pattern: "redis-set-no-ttl",
            fix: "SET without TTL = unbounded memory growth. Use SETEX or SET ... EX <seconds>.",
            line: 0,
        });
    }
    if hit(8, content) {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P0Block,
            store: Store::Kv,
            pattern: "redis-keys-glob",
            fix: "KEYS * blocks Redis (O(n)). Use SCAN cursor iteration.",
            line: 0,
        });
    }
    if hit(9, content) {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P0Block,
            store: Store::Kv,
            pattern: "dynamodb-scan",
            fix: "Scan reads entire table. Use Query with partition key + sort condition.",
            line: 0,
        });
    }
}
