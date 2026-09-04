//! Migration pipeline orchestrator and module exports.
#[path = "migrate/export.rs"]
pub mod export;
#[path = "migrate/import.rs"]
pub mod import;
#[path = "migrate/types.rs"]
pub mod types;
#[path = "migrate/validate.rs"]
pub mod validate;

#[cfg(test)]
#[path = "migrate/tests.rs"]
mod tests;

pub use export::{export_from_surreal, normalize_record};
pub use import::{import_records, read_ndjson};
pub use types::{
    EdgeRecord, EntityRecord, EventRecord, ImportOpts, MemoryRecord, MigrateReport,
    NormalizedRecord, PartRecord, ProjectRecord, SourceCounts, ValidateReport,
};
pub use validate::validate_store;
