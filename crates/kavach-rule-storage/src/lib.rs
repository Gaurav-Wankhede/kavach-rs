//! Rule storage: persist and load TOON skill rules from disk.

pub mod error;
pub mod index;
pub mod loader;
pub mod registry;
pub mod registry_builder;
pub mod registry_cache;
pub mod store;
pub mod version;
pub mod writer;

pub use error::StorageError;
pub use index::RuleIndex;
pub use registry::{RegistryEntry, SkillRegistry};
pub use registry_builder::build_from_rules;
pub use registry_cache::{is_stale, load_or_rebuild, load_registry, save_registry};
pub use store::{RuleStore, StoredRule};
pub use version::RuleVersion;
