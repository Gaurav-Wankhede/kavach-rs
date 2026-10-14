//! Migration types and DTO representations for NDJSON export/import.
use serde::{Deserialize, Serialize};

/// Default weight for relationship edges when omitted in raw records.
const fn default_weight() -> f64 {
    1.0
}

/// One memory row in export NDJSON format.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct MemoryRecord {
    pub project: String,
    pub category: String,
    pub entry_key: String,
    pub title: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub entry_status: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub decay_score: Option<f64>,
    #[serde(default)]
    pub access_count: i64,
    #[serde(default)]
    pub priority: Option<i64>,
    #[serde(default)]
    pub lane: Option<String>,
    #[serde(default)]
    pub exec_prompt: Option<String>,
    #[serde(default)]
    pub occupied_by: Option<String>,
    #[serde(default)]
    pub occupied_until: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Knowledge graph entity in export NDJSON format.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct EntityRecord {
    pub id: String,
    pub entity_type: String,
    pub name: String,
    #[serde(default)]
    pub properties: Option<String>,
    #[serde(default)]
    pub content_hash: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
}

/// Directed knowledge graph edge in export NDJSON format.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct EdgeRecord {
    pub from_id: String,
    pub rel_type: String,
    pub to_id: String,
    #[serde(default = "default_weight")]
    pub weight: f64,
}

/// Project registry record in export NDJSON format.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProjectRecord {
    pub slug: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Project part record in export NDJSON format.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct PartRecord {
    pub project: String,
    pub part: String,
    #[serde(default)]
    pub path: Option<String>,
}

/// Event log record in export NDJSON format.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct EventRecord {
    #[serde(default)]
    pub id: Option<i64>,
    pub project: String,
    pub kind: String,
    #[serde(default)]
    pub payload: Option<String>,
    pub created_at: String,
}

/// Sum-type container representing any valid record in the migration stream.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum NormalizedRecord {
    Memory(Box<MemoryRecord>),
    Entity(EntityRecord),
    Edge(EdgeRecord),
    Project(ProjectRecord),
    Part(PartRecord),
    Event(EventRecord),
}

impl NormalizedRecord {
    /// Increment the matching counter in SourceCounts.
    pub const fn count_into(&self, counts: &mut SourceCounts) {
        match self {
            Self::Memory(_) => counts.memory = counts.memory.saturating_add(1),
            Self::Entity(_) => counts.entity = counts.entity.saturating_add(1),
            Self::Edge(_) => counts.edge = counts.edge.saturating_add(1),
            Self::Project(_) => counts.project = counts.project.saturating_add(1),
            Self::Part(_) => counts.part = counts.part.saturating_add(1),
            Self::Event(_) => counts.event = counts.event.saturating_add(1),
        }
    }
}

/// Options controlling how the migration import runs.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ImportOpts {
    pub batch_size: usize,
    pub overwrite: bool,
    pub dry_run: bool,
}

impl ImportOpts {
    #[must_use]
    pub const fn new(batch_size: usize, overwrite: bool, dry_run: bool) -> Self {
        Self {
            batch_size,
            overwrite,
            dry_run,
        }
    }
}

impl Default for ImportOpts {
    fn default() -> Self {
        Self::new(500, false, false)
    }
}

/// Summary report emitted after an import pass finishes.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MigrateReport {
    pub memory_count: usize,
    pub entity_count: usize,
    pub edge_count: usize,
    pub project_count: usize,
    pub part_count: usize,
    pub event_count: usize,
    pub warnings: Vec<String>,
}

/// Summary counts tallied from source stream for pre/post validation.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct SourceCounts {
    pub memory: usize,
    pub entity: usize,
    pub edge: usize,
    pub project: usize,
    pub part: usize,
    pub event: usize,
}

impl SourceCounts {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            memory: 0,
            entity: 0,
            edge: 0,
            project: 0,
            part: 0,
            event: 0,
        }
    }
}

/// Verification report returned by the post-migration validator.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ValidateReport {
    pub memory_count: usize,
    pub entity_count: usize,
    pub edge_count: usize,
    pub dangling_edges: Vec<String>,
    pub fts_matched: bool,
    pub is_clean: bool,
}
