//! Irreversible-action gate: detects schema-destroying SQL and critical-path writes.
//!
//! `P1Advisory` only — the host gate emits `[IRREVERSIBLE]` context per
//! kavach-engine/CLAUDE.md severity policy.
//! SOURCE: <https://github.com/Dicklesworthstone/destructive_command_guard>

mod rules;

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum IrreversibleSeverity {
    P1Advisory,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct IrreversibleHit {
    pub severity: IrreversibleSeverity,
    pub pattern: &'static str,
    pub fix: &'static str,
    pub line: usize,
}

#[must_use]
pub fn detect(file_path: &str, content: &str) -> Vec<IrreversibleHit> {
    if content.is_empty() || crate::file_types::is_test_file(file_path) {
        return vec![];
    }
    let mut hits = Vec::new();
    for (re, pattern, fix) in rules::path_rules() {
        if re.is_match(file_path) {
            hits.push(IrreversibleHit {
                severity: IrreversibleSeverity::P1Advisory,
                pattern,
                fix,
                line: 0,
            });
        }
    }
    let path_obj = Path::new(file_path);
    let is_sql_file = path_obj
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("sql"));
    let is_cassandra_file = path_obj
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("cql"));
    let scan_sql = is_sql_file
        || is_cassandra_file
        || content.contains("sqlx::query")
        || content.contains("db.query(");
    if scan_sql {
        for (i, line) in content.lines().enumerate() {
            for rule in rules::sql_rules() {
                if rule.re.is_match(line) {
                    hits.push(IrreversibleHit {
                        severity: IrreversibleSeverity::P1Advisory,
                        pattern: rule.pattern,
                        fix: rule.fix,
                        line: i.saturating_add(1),
                    });
                }
            }
        }
    }
    hits
}

#[cfg(test)]
#[path = "irreversible_guard_tests.rs"]
mod tests;
