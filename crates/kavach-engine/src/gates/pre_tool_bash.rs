//! hub: `pre_tool_bash` dispatch.
//!
//! `handle_bash` routes Bash commands through the kavach-CLI fast-path, the
//! stateless `blocklist`, then the stateful `advisory_ctx` tail. The quote-aware
//! `strip_quoted_regions` primitive (shared with sibling gates) lives in `quote`.
mod advisories;
mod advisory_ctx;
mod blocklist;
mod decision;
mod dispatch;
mod quote;
mod test_tracker;
mod write_bypass;

pub(crate) use dispatch::handle_bash;
pub(crate) use quote::strip_quoted_regions;
