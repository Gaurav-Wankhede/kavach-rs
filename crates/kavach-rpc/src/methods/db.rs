//! db.* RPC methods — maps to kavach db CLI commands.
//!
//! DECOMPOSED per Rust module organization best practice
//! (Rust Book ch07, `LogRocket` web services patterns).
//! Thin hub + micro-file leaves (≤100 LOC each), following `rust_guard.rs` template.
//! All public API stable — zero change to rpc.rs callers.

mod archive;
mod delete;
mod event;
mod expire;
mod find;
mod get;
mod graph_fetch;
mod graph_query;
mod kanban;
mod kanban_close;
mod list_parts;
mod list_projects;
mod bandit_backfill;
mod ope;
mod ope_audit;
mod query;
mod register;
mod register_part;
mod rotate;
mod search;
mod set_parent;
mod set_priority;
mod status_update;
mod tree;
mod util;
mod wipe_project;
mod write;

pub use archive::{ArchiveParams, ArchiveResult, archive};
pub use delete::{DeleteParams, DeleteResult, delete, delete_confirm_phrase};
pub use event::{BanditRowParams, BanditRowResult, EventParams, EventResult, bandit_row, event};
pub use bandit_backfill::{BanditBackfillParams, BanditBackfillResult, bandit_backfill_session};
pub use ope::{OpeEvaluateParams, OpeEvaluateResult, ope_evaluate};
pub use ope_audit::{OpeAuditParams, OpeAuditResult, ope_audit};
pub use expire::{ExpireParams, ExpireResult, expire};
pub use find::{FindParams, FindResult, find_part, find_project};
pub use get::{GetEntry, GetParams, GetResult, get};
pub use graph_fetch::{GraphEdge, GraphFetchParams, GraphFetchResult, GraphNode, graph_fetch};
pub use graph_query::{GraphQueryParams, GraphQueryResult, graph_query};
pub use kanban::{KanbanCounts, KanbanItem, KanbanParams, KanbanResult, kanban};
pub use kanban_close::{KanbanCloseParams, KanbanCloseResult, kanban_close};
pub use list_parts::{ListPartsParams, ListPartsResult, PartRow, list_parts};
pub use list_projects::{ListProjectsParams, ListProjectsResult, ProjectRow, list_projects};
pub use query::{QueryEntry, QueryParams, QueryResult, query};
pub use register::{RegisterParams, RegisterResult, register};
pub use register_part::{RegisterPartParams, RegisterPartResult, register_part};
pub use rotate::{RotateParams, RotateResult, rotate};
pub use search::{SearchHit, SearchParams, SearchResult, search};
pub use set_parent::{SetParentParams, SetParentResult, set_parent};
pub use set_priority::{SetPriorityParams, SetPriorityResult, set_priority};
pub use status_update::{StatusUpdateParams, StatusUpdateResult, status_update};
pub use tree::{TreeNode, TreeParams, TreeResult, tree};
pub use wipe_project::{
    WipeProjectParams, WipeProjectResult, WipeReportDto, wipe_confirm_phrase, wipe_project,
};
pub use write::{WriteParams, WriteResult, write};

#[cfg(test)]
#[path = "db_test.rs"]
mod tests;
