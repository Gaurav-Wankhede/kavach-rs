//! Algorithm decision recorder — fires after Write/Edit on .rs files.
//!
//! Extracts the structured `// ALGO:` comment block from written content,
//! upserts the decision into `algorithm_decisions`, appends a kavach-db event,
//! writes graph edges (file→algorithm, `algorithm→problem_class`), and triggers
//! `kavach rag enrich` on the arch skill directory.
//!
//! All operations are fire-and-forget — a recorder failure never blocks the
//! post-write gate. Errors are silently dropped.
//!
//! hub: re-exports `record`; parsing, datetime helpers, verification, and the
//! persistence pipeline live in submodules.
mod datetime;
mod parse;
mod record;
mod verify;

#[cfg(test)]
mod tests;

pub(in crate::gates) use record::record;
