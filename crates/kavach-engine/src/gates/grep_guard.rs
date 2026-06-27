//! Detect dangerous grep patterns that cause 30+ minute hangs.
//!
//! Problems caught:
//!  1. `grep -r` without --exclude-dir for .`git/target/node_modules`
//!  2. `grep -r` scanning binary files (missing -I flag)
//!  3. `grep -r` on large paths without file-type filters
//!  4. Using `grep` in Bash when ripgrep (rg) is available
//!
//! TOOLBELT: Recommends `rg` (ripgrep) — 5-13x faster, auto-skips .git/binaries.
//! SOURCE: <https://www.codeant.ai/blogs/ripgrep-vs-grep-performance>
//!
//! `detect` holds the command classifier; `messages` builds the advisory text.
mod detect;
mod messages;
#[cfg(test)]
#[path = "grep_guard_test.rs"]
#[path = "grep_guard_test.rs"]
mod tests;
pub(crate) use detect::check_grep_command;
pub(crate) use messages::origin_pointer;
