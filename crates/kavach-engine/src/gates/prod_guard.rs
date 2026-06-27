//! Production operations guard — warns on high-risk commands, BLOCKS destructive
//! ones. Two-tier: `check_prod_destructive` HARD BLOCKS, `check_prod_ops` warns.
//! SOURCE: Pocket OS incident (April 2026) — AI agent deleted prod DB in 9 seconds
//! <https://blog.railway.com/p/your-ai-wants-to-nuke-your-database>
//!
//! Detection (`detect`) routes the matcher's view per command class: DB-client
//! and shell `-c` payloads stay verbatim; other tools' quoted args are stripped.
mod destructive;
mod detect;
mod ops;
#[cfg(test)]
#[path = "prod_guard_test.rs"]
#[cfg(test)]
#[path = "prod_guard_test.rs"]
mod tests;
pub(crate) use destructive::check_prod_destructive;
pub(crate) use ops::check_prod_ops;
