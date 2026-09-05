//! TursoStore — embedded libSQL storage backend (PRD_SURREALDB_TO_TURSO_MIGRATION §3).
//!
//! One SQLite-compatible file at `~/.kavach/kavach.db` (WAL mode), replacing
//! the standalone TursoStoreDB server. Layout (one domain per submodule, tests as
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
pub mod vector;

#[cfg(test)]
#[path = "turso/entries_test.rs"]
mod entries_test;

#[cfg(test)]
#[path = "turso/search_test.rs"]
mod search_test;

#[cfg(test)]
#[path = "turso/graph_test.rs"]
mod graph_test;

#[cfg(test)]
#[path = "turso/vector_test.rs"]
mod vector_test;

pub const DEFAULT_DB_FILENAME: &str = "kavach.db";

/// Categories mirror the TursoStoreDB SCHEMAFULL per-category tables. A single
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
              occupied_epoch INTEGER,
              occupied_heartbeat TEXT,
              harness       TEXT,
              workflow_path TEXT,
              feedback      TEXT,
              expires_at    TEXT,
              created_at    TEXT NOT NULL,
              updated_at    TEXT NOT NULL,
              embedding     F32_BLOB(384),
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
    "CREATE TABLE IF NOT EXISTS projects (
              slug TEXT PRIMARY KEY,
              path TEXT UNIQUE,
              tags TEXT NOT NULL DEFAULT '[]',
              display TEXT,
              workdir TEXT,
              stack TEXT,
              aliases TEXT,
              parent TEXT,
              updated_at TEXT
            )",
    "CREATE TABLE IF NOT EXISTS parts (
              project     TEXT NOT NULL,
              part_name   TEXT NOT NULL,
              part_path   TEXT NOT NULL,
              part_type   TEXT NOT NULL DEFAULT 'dir',
              stack       TEXT,
              description TEXT,
              updated_at  TEXT NOT NULL,
              PRIMARY KEY (project, part_name)
            )",
    "CREATE TABLE IF NOT EXISTS events (id INTEGER PRIMARY KEY AUTOINCREMENT, project TEXT NOT NULL, kind TEXT NOT NULL, payload TEXT, created_at TEXT NOT NULL)",
    // Citations table for official-docs context (Phase 6).
    "CREATE TABLE IF NOT EXISTS citations (
              project       TEXT NOT NULL,
              entry_key     TEXT NOT NULL,
              name          TEXT NOT NULL,
              slug          TEXT NOT NULL,
              url           TEXT NOT NULL,
              metadata      TEXT NOT NULL DEFAULT '[]',
              access_count  INTEGER NOT NULL DEFAULT 0,
              created_at    TEXT NOT NULL,
              updated_at    TEXT NOT NULL,
              PRIMARY KEY (project, entry_key)
            )",
    // Gate config overrides (Phase 6).
    "CREATE TABLE IF NOT EXISTS gate_config (
              project       TEXT NOT NULL,
              gate_key      TEXT NOT NULL,
              kind          TEXT NOT NULL,
              value_num     REAL,
              value_bool    INTEGER,
              value_list    TEXT,
              value_text    TEXT,
              updated_at    TEXT NOT NULL,
              PRIMARY KEY (project, gate_key)
            )",
    // BM25 corpus (Phase 2).
    "CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(entry_key UNINDEXED, project UNINDEXED, category UNINDEXED, title, content, tokenize='porter unicode61')",
    // RLVR bandit log (harness-rl): content-addressed decision rows.
    "CREATE TABLE IF NOT EXISTS bandit_log (
              key          TEXT PRIMARY KEY,
              payload      TEXT NOT NULL,
              created_at   TEXT NOT NULL
            )",
    "CREATE INDEX IF NOT EXISTS idx_bandit_log_created ON bandit_log(created_at)",
    // Algo/arch decision records (keyed UPSERT, deterministic blake3 keys).
    "CREATE TABLE IF NOT EXISTS algo_decision (
              id           TEXT PRIMARY KEY,
              project      TEXT NOT NULL,
              problem_class TEXT NOT NULL,
              chosen       TEXT NOT NULL,
              time_complexity TEXT NOT NULL,
              space_complexity TEXT NOT NULL,
              file_path    TEXT NOT NULL,
              search_year  INTEGER NOT NULL,
              search_month INTEGER NOT NULL
            )",
    "CREATE INDEX IF NOT EXISTS idx_algo_decision_project ON algo_decision(project)",
    "CREATE TABLE IF NOT EXISTS arch_decision (
              id           TEXT PRIMARY KEY,
              project      TEXT NOT NULL,
              pattern      TEXT NOT NULL,
              scope        TEXT NOT NULL,
              cap_choice   TEXT,
              failure_mode TEXT NOT NULL,
              tradeoff     TEXT NOT NULL,
              file_path    TEXT NOT NULL,
              search_year  INTEGER NOT NULL,
              search_month INTEGER NOT NULL
            )",
    "CREATE INDEX IF NOT EXISTS idx_arch_decision_project ON arch_decision(project)",
    // Gate-pattern store (self-improving gates): tokenized error signatures.
    "CREATE TABLE IF NOT EXISTS gate_pattern (
              id              TEXT PRIMARY KEY,
              project         TEXT NOT NULL,
              tool_name       TEXT NOT NULL DEFAULT '',
              gate_name       TEXT NOT NULL DEFAULT '',
              error_tokens    TEXT NOT NULL,
              fix_strategy    TEXT NOT NULL,
              imperative_rewrite TEXT NOT NULL,
              dsa_rationale   TEXT NOT NULL,
              occurrence_count INTEGER NOT NULL DEFAULT 1,
              bloom_bytes     BLOB,
              tier            TEXT NOT NULL DEFAULT 'research',
              updated_at      TEXT NOT NULL
            )",
    "CREATE INDEX IF NOT EXISTS idx_gate_pattern_project ON gate_pattern(project, tier)",
    // Run records (harness execution tracking).
    "CREATE TABLE IF NOT EXISTS run (
              id           INTEGER PRIMARY KEY AUTOINCREMENT,
              project      TEXT,
              entry_key    TEXT NOT NULL,
              branch       TEXT,
              status       TEXT NOT NULL,
              command      TEXT,
              pid          INTEGER,
              started_at   TEXT,
              finished_at  TEXT,
              exit_code    INTEGER,
              cost_usd     REAL
            )",
    "CREATE INDEX IF NOT EXISTS idx_run_project ON run(project, started_at)",
    // RAG tree blobs (NDJSON tree per source label).
    "CREATE TABLE IF NOT EXISTS rag_tree (
              source      TEXT PRIMARY KEY,
              built_at    TEXT NOT NULL,
              tree_json   BLOB NOT NULL,
              source_hash TEXT NOT NULL,
              source_dir  TEXT NOT NULL DEFAULT '',
              updated_at  TEXT NOT NULL
            )",
    // NanoLM live-doc corpus (BM25 via FTS5).
    "CREATE TABLE IF NOT EXISTS nlm_doc (
              source_url  TEXT NOT NULL,
              heading     TEXT NOT NULL,
              body        TEXT NOT NULL,
              captured_at TEXT NOT NULL,
              updated_at  TEXT NOT NULL,
              PRIMARY KEY (source_url, heading)
            )",
    "CREATE VIRTUAL TABLE IF NOT EXISTS nlm_doc_fts USING fts5(source_url UNINDEXED, heading, body, tokenize='porter unicode61')",
    // Durable harness runtime state (session_runtime): one row per session.
    "CREATE TABLE IF NOT EXISTS session_runtime (
              session_id TEXT PRIMARY KEY,
              workdir    TEXT NOT NULL,
              state_blob TEXT NOT NULL,
              updated_at TEXT NOT NULL
            )",
    // Bulk-edit authority (single-RCA-bound batch edits): one row per sweep.
    "CREATE TABLE IF NOT EXISTS bulk_manifest (
              id                  INTEGER PRIMARY KEY AUTOINCREMENT,
              sweep_id            TEXT NOT NULL UNIQUE,
              project             TEXT NOT NULL,
              root_rca            TEXT NOT NULL,
              scope_glob          TEXT NOT NULL,
              lint_class          TEXT NOT NULL,
              fix_strategy        TEXT NOT NULL,
              blast_estimate      INTEGER NOT NULL,
              signed_by_session   TEXT NOT NULL,
              approved_by         TEXT NOT NULL,
              approved_at         TEXT NOT NULL,
              expires_at          TEXT NOT NULL,
              conformance_applied INTEGER NOT NULL DEFAULT 0,
              conformance_refused INTEGER NOT NULL DEFAULT 0,
              conformance_drifted INTEGER NOT NULL DEFAULT 0,
              status              TEXT NOT NULL DEFAULT 'active',
              closed_at           TEXT
            )",
    "CREATE INDEX IF NOT EXISTS idx_bulk_manifest_status ON bulk_manifest(status)",
];

pub(crate) fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}
