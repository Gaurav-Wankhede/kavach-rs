//! Vector database-specific detection.

use super::pattern_set::hit;
use super::types::{DbOpsSeverity, DbOpsViolation, Store};

pub(super) fn detect(content: &str, violations: &mut Vec<DbOpsViolation>) {
    if hit(11, content)
        && !content.contains("dimension")
        && !content.contains("DIMENSION")
        && !content.contains("dim ")
    {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P1Advisory,
            store: Store::Vector,
            pattern: "vector-upsert-no-dim-check",
            fix: "Assert vector.len() == EXPECTED_DIM before upsert.",
            line: 0,
        });
    }
    if hit(12, content)
        && !content.contains("namespace")
        && !content.contains("filter")
        && !content.contains("tenant")
    {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P0Block,
            store: Store::Vector,
            pattern: "vector-query-no-tenant",
            fix: "Vector query without namespace/filter leaks across tenants. Add namespace=tenant_id.",
            line: 0,
        });
    }
}
