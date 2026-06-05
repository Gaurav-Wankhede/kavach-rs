// Post-migration footprint: only `MemoryEntry` survives — used by `read.rs`
// to deserialize cross-category SELECTs into a single Rust shape.
//
// Removed 2026-05 (no callers):
//   - `DualWriter` struct + `new`/`write_decision`/`write_research`/
//     `write_roadmap`/`write_entity`/`log_write` (SQLite→Surreal dual-write
//     bridge that ran during the one-shot migration).
//   - `migrate_memory_entry` (one-shot per-category insert helper).
//
// Module name kept (rather than renamed to `memory_entry.rs`) to avoid
// touching every caller's `use kavach_surreal::dual_write::MemoryEntry;`
// import path. The rename is a separate cosmetic unit.
use serde::{Deserialize, Serialize};
use surrealdb_types::{RecordId, SurrealValue};

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate struct-literal DTO (kavach-rpc); non_exhaustive => E0639"
)]
pub struct MemoryEntry {
    pub id: Option<RecordId>,
    pub project: RecordId,
    // SurrealDB 3.0 deserializes query results via `SurrealValue`, which does
    // NOT honor `#[serde(default)]` and has no `#[surreal(default)]` knob — the
    // ONLY missing-field tolerance it supports is `Option<T>`. SCHEMAFULL
    // per-category tables don't all define every legacy field, so a
    // cross-category SELECT hands back `none` for a field a different category
    // carries. Model these as `Option<String>`; readers use the accessors below.
    pub category: Option<String>,
    pub entry_key: String,
    pub title: String,
    pub content: String,
    pub status: Option<String>,
    pub entry_status: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub decay_score: Option<f64>,
    #[serde(default)]
    pub access_count: Option<i64>,
    #[serde(default)]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    // Lower number = higher dispatch rank. NULL = unprioritized (sorts last
    // via NULLS LAST in read.rs::list_by_project). Defined on roadmap +
    // decision tables only; selected as `priority` in MEMORY_FIELDS, the
    // cross-category SELECT yields NONE for tables without the column.
    // Stored as i64 to satisfy SurrealValue derive; callers wrap in Priority
    // newtype at the boundary for validation bounds [0, 1000].
    #[serde(default)]
    pub priority: Option<i64>,
}

impl MemoryEntry {
    /// Category as `&str`, empty when the row's table omits the field.
    #[must_use]
    pub const fn category_str(&self) -> &str {
        match &self.category {
            Some(s) => s.as_str(),
            None => "",
        }
    }

    /// Decision/research status as `&str`, empty when absent.
    #[must_use]
    pub const fn status_str(&self) -> &str {
        match &self.status {
            Some(s) => s.as_str(),
            None => "",
        }
    }

    /// Roadmap/kanban lifecycle status as `&str`, empty when absent.
    #[must_use]
    pub const fn entry_status_str(&self) -> &str {
        match &self.entry_status {
            Some(s) => s.as_str(),
            None => "",
        }
    }
}
