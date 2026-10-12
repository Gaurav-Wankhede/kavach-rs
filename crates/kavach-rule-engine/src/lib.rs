//! Unified Rule Engine for evaluating TOON skill rules during hook execution.
//! Incorporates AST models, parsing, disk storage, and compliance enforcement.

pub mod ast;
pub mod parser;
pub mod storage;

pub use ast::{
    SkillDefinition, SkillMetadata, SkillPriority,
    ResearchGate, ErrorHandling, PendingTasks,
};
pub use parser::{
    parse_frontmatter, extract_sections, FrontmatterMetadata,
};
pub use storage::{
    RuleIndex, SkillRegistry, RuleStore, StoredRule, RuleVersion, StorageError,
    build_from_rules, is_stale, load_or_rebuild, load_registry, save_registry,
};


pub mod compliance;
pub mod context;
pub mod enforce;
pub mod engine;
pub mod file_matcher;
pub mod matcher;
pub mod research;
pub mod result;

pub use context::EvalContext;
pub use engine::RuleEngine;
pub use result::{RuleAction, RuleResult};
