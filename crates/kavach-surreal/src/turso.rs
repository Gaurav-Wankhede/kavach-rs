//! TursoStore — embedded libSQL storage backend (PRD_SURREALDB_TO_TURSO_MIGRATION §3).
//!
//! One SQLite-compatible file at `~/.kavach/kavach.db` (WAL mode), replacing
//! the standalone SurrealDB server. Layout (one domain per submodule, tests as
//! `<file>_test.rs` siblings):
//!
//! - [`connection`] — open/close lifecycle + schema bootstrap
//! - [`entries`]    — memory-entry CRUD (`EntryStore` trait)
//! - [`search`]     — FTS5 BM25 rank lists + RRF fusion (`SearchStore`)
//! - [`graph`]      — entity/edge store + traversal (`GraphStore`)
//!
//! Repo law allows ONE inherent impl block per type, so lifecycle lives here;
//! domain operations are extension traits implemented in their submodules.

pub mod connection;
pub mod entries;
pub mod graph;
pub mod migrate;
pub mod search;

#[cfg(test)]
#[path = "turso/entries_test.rs"]
mod entries_test;

#[cfg(test)]
#[path = "turso/search_test.rs"]
mod search_test;

#[cfg(test)]
#[path = "turso/graph_test.rs"]
mod graph_test;

pub const DEFAULT_DB_FILENAME: &str = "kavach.db";

/// Categories mirror the SurrealDB SCHEMAFULL per-category tables. A single
/// table + category discriminator replaces them; CHECK keeps garbage out.
pub const CATEGORIES: [&str; 5] = ["decision", "research", "roadmap", "pattern", "app_spec"];

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TursoError {
    #[error("libsql: {0}")]
    Libsql(#[from] libsql::Error),
    #[error("row mapping: {0}")]
    Mapping(String),
    #[error("migrate: {0}")]
    Migrate(String),
}

pub type TursoResult<T> = Result<T, TursoError>;

/// Embedded handle. Cloneable; each clone opens its own libSQL connection to
/// the same file (WAL allows concurrent readers + single writer w/ timeout).
#[derive(Clone)]
pub struct TursoStore {
    pub(crate) conn: libsql::Connection,
}

impl std::fmt::Debug for TursoStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TursoStore").finish_non_exhaustive()
    }
}

/// DDL for a fresh database. NOTE: libsql execute() runs exactly ONE statement
/// per call — every statement must be executed individually.
pub(crate) const SCHEMA_DDL: &[&str] = &[
    // Core memory rows (replaces the five SCHEMAFULL category tables).
    "CREATE TABLE IF NOT EXISTS memories (
              project       TEXT NOT NULL,
              category      TEXT NOT NULL CHECK (category IN ('decision','research','roadmap','pattern','app_spec')),
              entry_key     TEXT NOT NULL,
              title         TEXT NOT NULL,
              content       TEXT NOT NULL DEFAULT '',
              status        TEXT,
              entry_status  TEXT,
              tags          TEXT NOT NULL DEFAULT '[]',
              decay_score   REAL,
              access_count  INTEGER NOT NULL DEFAULT 0,
              priority      INTEGER,
              lane          TEXT,
              exec_prompt   TEXT,
              occupied_by   TEXT,
              occupied_until TEXT,
              created_at    TEXT NOT NULL,
              updated_at    TEXT NOT NULL,
              embedding     F32_BLOB(768),
              PRIMARY KEY (project, category, entry_key)
            )",
    "CREATE INDEX IF NOT EXISTS idx_memories_status ON memories(project, status)",
    // Simple dependency edges between memory rows (roadmap DAG parity).
    "CREATE TABLE IF NOT EXISTS edges (
              src_project TEXT NOT NULL, src_cat TEXT NOT NULL, src_key TEXT NOT NULL,
              dst_project TEXT NOT NULL, dst_cat TEXT NOT NULL, dst_key TEXT NOT NULL,
              kind TEXT NOT NULL CHECK (kind IN ('depends_on','blocks','supersedes','references','mentions')),
              created_at TEXT NOT NULL,
              PRIMARY KEY (src_project, src_cat, src_key, dst_project, dst_cat, dst_key, kind)
            )",
    "CREATE INDEX IF NOT EXISTS idx_edges_dst ON edges(dst_project, dst_cat, dst_key)",
    "CREATE TABLE IF NOT EXISTS projects (slug TEXT PRIMARY KEY, path TEXT UNIQUE, tags TEXT NOT NULL DEFAULT '[]')",
    "CREATE TABLE IF NOT EXISTS parts (project TEXT NOT NULL, part TEXT NOT NULL, path TEXT, PRIMARY KEY (project, part))",
    "CREATE TABLE IF NOT EXISTS events (id INTEGER PRIMARY KEY AUTOINCREMENT, project TEXT NOT NULL, kind TEXT NOT NULL, payload TEXT, created_at TEXT NOT NULL)",
    // BM25 corpus (Phase 2).
    "CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(entry_key UNINDEXED, project UNINDEXED, category UNINDEXED, title, content, tokenize='porter unicode61')",
];

/// The Surreal DTO requires a `RecordId` for `project`; until the caller
/// refactor lands, synthesize one from the slug string.
pub(crate) fn fake_record_id(slug: &str) -> surrealdb_types::RecordId {
    surrealdb_types::RecordId::new(
        "project",
        surrealdb_types::RecordIdKey::String(slug.to_owned()),
    )
}

pub(crate) fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}
