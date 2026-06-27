//! Phase C bridge between kavach-db `rag_trees` storage and the kavach-rag
//! matcher. Runs purely read-only in hook hot paths.
//!
//! The router is **advisory** — it never blocks or denies. It loads a persisted
//! tree by label, runs the matcher, and returns a context block that the
//! `pre_write` gate appends alongside existing skill enforcement. Any failure
//! (missing db row, parse error, empty result) degrades to an empty string so
//! the gate never fails because rag is unavailable.
//!
//! A per-process `OnceLock` cache per label (see `cache`) avoids re-opening the
//! `SurrealDB` connection and re-parsing the tree JSON when multiple gates fire
//! within the same hook invocation.
//!
//! hub: re-exports the advisory + skill-routing entry points; the cache, RPC
//! wrappers, advisory formatting, and skill routing live in submodules.
mod advisory;
mod cache;
mod rpc;
mod skills;
#[cfg(test)]
#[path = "rag_router_test.rs"]
mod tests;
pub(crate) use advisory::advisory_context_all;
pub(crate) use skills::{SKILL_MATCH_FLOOR, SkillMatch, top_skill_match, top_skill_names_all};
