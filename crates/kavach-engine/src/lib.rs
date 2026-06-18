#![expect(
    clippy::redundant_pub_crate,
    reason = "nursery lint conflicts with workspace unreachable_pub=deny: pub(crate) items in private gate modules satisfy unreachable_pub; redundant_pub_crate's pub suggestion would re-trigger it"
)]
#![expect(
    clippy::print_stderr,
    reason = "kavach-engine is the Claude Code hook engine; no tracing dep by design, stderr IS the hook log channel surfaced to the harness"
)]

pub mod error;
pub mod gate_runner;
pub mod gates;
pub mod graph_infer;
pub mod team;
pub mod toolbelt;

pub use error::EngineError;
pub use gate_runner::run_gate;
pub use gates::event_log::{
    ExtractedRelationship, extract_memory_entry_references, extract_memory_entry_relationships,
    memory_entry_qualified_name, project_memory_entry_rpc,
};
pub use gates::status_gate::{StatusGateVerdict, verify_status_promotion};
pub use graph_infer::{InferRow, InferredRel, infer_relationships};
pub use team::{DagScheduler, DispatchPlan, Spawner, SpawnerKind, TeamDispatchError};
