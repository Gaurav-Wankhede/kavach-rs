//! kavach-rpc — in-process JSON-RPC 2.0 dispatch over the kavach `SurrealDB` server.
//!
//! There is no standalone RPC daemon any more: the database is owned by a
//! `surreal start` server (launchd `ai.shared.kavach-surreal`) and every kavach
//! process is a thin ws client. `client::call` opens a ws connection, builds the
//! `RpcModule` from `rpc::build_module`, and dispatches requests in-process via
//! `raw_json_request` — no socket, no transport layer. The method registry lives
//! under `methods/` and is wired in `rpc.rs`.
pub mod client;
pub mod error;
pub mod methods;
pub mod rpc;
pub mod state;
