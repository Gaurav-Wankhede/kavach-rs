//! Types for database operation violations.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "matched cross-crate in kavach-rpc gates.rs; non_exhaustive => E0004"
)]
pub enum DbOpsSeverity {
    P0Block,
    P1Advisory,
    P2Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "matched cross-crate in kavach-rpc gates.rs; non_exhaustive => E0004"
)]
pub enum Store {
    Sql,
    NoSql,
    Kv,
    Graph,
    Vector,
    Cloudflare,
    Unknown,
}

#[derive(Debug, Clone)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed cross-crate by callers building violations; non_exhaustive => E0639"
)]
pub struct DbOpsViolation {
    pub severity: DbOpsSeverity,
    pub store: Store,
    pub pattern: &'static str,
    pub fix: &'static str,
    pub line: usize,
}
