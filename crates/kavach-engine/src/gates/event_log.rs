//! Event logging + graph projection subsystem.
//!
//! Wraps the `event.append` RPC with silent failure, then fires graph
//! projections (file→skill, session→project, gate→memory, session→skill). All
//! paths go through `kavach_rpc` to the SurrealDB-backed daemon — gates must
//! never fail because event logging is unavailable.
//!
//! Decomposed by responsibility: `rpc` (RPC primitives), `refs` (content/file
//! reference extraction), `relationships` (typed frontmatter + wikilink rels),
//! `projections` (event→graph-edge projections), and `loggers` (the public
//! gate-facing `log_*` entry points).

mod loggers;
mod projections;
mod refs;
mod relationships;
mod rpc;

// Public API (re-exported at crate root via lib.rs).
pub use projections::project_memory_entry_rpc;
pub use refs::{extract_memory_entry_references, memory_entry_qualified_name};
pub use relationships::{ExtractedRelationship, extract_memory_entry_relationships};

// Gate-facing loggers (pub(crate)).
// SOURCE: decision.model-shift-router-advisory
pub(crate) use loggers::{
    ToolFailureLog, log_file_write, log_gate_decision, log_intent, log_model_route, log_session,
    log_skill_invoke, log_tool_failure,
};
