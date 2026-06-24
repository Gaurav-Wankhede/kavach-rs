// RPC verbs for bulk-mode (single-RCA-bound batch edits).
// SOURCE: roadmap.unit.kavach-bulk-mode acceptance #2.
mod bump;
mod close;
mod create;
mod get;
mod list;

pub use bump::{BumpParams, BumpResult, bump};
pub use close::{CloseParams, CloseResult, close};
pub use create::{CreateResult, CreateRpcParams, create};
pub use get::GetResult;
pub use list::{ListActiveParams, ListActiveResult, list_active_rpc};
