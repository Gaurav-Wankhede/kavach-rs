//! `Database Operations Gate` — `SQL` / `NoSQL` / `KV` / `Graph` / `Vector`
//!
//! Covers ACID, RLS, CRUD, scalability, durability across 5 store families.
//! Behavior-preserving split: hub orchestrator with leaf detectors per store type.
//!
//! SOURCES (verified 2026-05):
//! - <https://www.postgresql.org/docs/current/ddl-rowsecurity.html>
//! - <https://www.postgresql.org/docs/current/transaction-iso.html>
//! - <https://www.mongodb.com/docs/manual/reference/write-concern>/
//! - <https://redis.io/docs/latest/commands/keys>/
pub use self::types::{DbOpsSeverity, DbOpsViolation, Store};
mod cf_compute;
mod cf_kv_d1;
mod graph;
mod helpers;
mod kv;
mod nosql;
mod pattern_set;
mod regex_builders;
mod sql;
mod types;
mod vector;
use helpers::{classify_store, is_db_file};
use pattern_set::hit;
#[must_use]
pub fn detect(file_path: &str, content: &str) -> Vec<DbOpsViolation> {
    if !is_db_file(file_path, content) {
        return vec![];
    }
    if crate::file_types::is_test_file(file_path) {
        return vec![];
    }
    let mut violations = Vec::new();
    let store = classify_store(file_path, content);
    if matches!(store, Store::Sql | Store::Unknown) {
        sql::detect(content, &mut violations);
    }
    if matches!(store, Store::NoSql) {
        nosql::detect(content, &mut violations);
    }
    if matches!(store, Store::Kv) {
        kv::detect(content, &mut violations);
    }
    if matches!(store, Store::Graph) {
        graph::detect(content, &mut violations);
    }
    if matches!(store, Store::Vector) {
        vector::detect(content, &mut violations);
    }
    if matches!(store, Store::Cloudflare) {
        cf_kv_d1::detect(content, &mut violations);
        cf_compute::detect(content, &mut violations);
    }
    if hit(13, content) {
        violations.push(DbOpsViolation {
            severity: DbOpsSeverity::P0Block,
            store,
            pattern: "n-plus-one",
            fix: "Query inside loop = N+1. Batch with WHERE id = ANY($1::uuid[]).",
            line: 0,
        });
    }
    violations
}
#[must_use]
pub fn block_count(file_path: &str, content: &str) -> usize {
    detect(file_path, content)
        .iter()
        .filter(|x| x.severity == DbOpsSeverity::P0Block)
        .count()
}
#[cfg(test)]
#[path = "database_ops_guard_test.rs"]
#[cfg(test)]
#[path = "database_ops_guard_test.rs"]
mod tests;
