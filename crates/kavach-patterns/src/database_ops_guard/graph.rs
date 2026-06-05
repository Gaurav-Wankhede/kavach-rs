//! Graph database-specific detection.

use super::pattern_set::{hit, matches_of};
use super::types::{DbOpsSeverity, DbOpsViolation, Store};

pub(super) fn detect(content: &str, violations: &mut Vec<DbOpsViolation>) {
    if hit(10, content) {
        let unbounded = matches_of(10, content)
            .into_iter()
            .any(|m| !m.as_str().contains(".."));
        if unbounded {
            violations.push(DbOpsViolation {
                severity: DbOpsSeverity::P0Block,
                store: Store::Graph,
                pattern: "cypher-unbounded-path",
                fix: "Variable-length [:REL*] without bound = exponential blowup. Use [:REL*1..5].",
                line: 0,
            });
        }
    }
}
