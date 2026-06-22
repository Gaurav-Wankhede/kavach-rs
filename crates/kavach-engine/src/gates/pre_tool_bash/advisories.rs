//! hub: Bash-command advisory checks (never P0 blocks). Each detector lives in
//! its own submodule and is quote-aware via the parent's `strip_quoted_regions`.
//! Re-exports the entry fns the `pre_tool_bash` dispatcher consumes.
mod cargo_flags;
mod commit;
mod git_add;
mod nextest;
mod secret_cli;
mod toolbelt_cli;

#[cfg(test)]
mod tests;

pub(super) use cargo_flags::check_multi_crate;
pub(super) use commit::check_commit_message;
pub(super) use git_add::is_git_add_all;
pub(super) use nextest::{check_nextest_advisory, scaffold_nextest_config};
pub(super) use secret_cli::check_secret_cli_read;
pub(super) use toolbelt_cli::check_toolbelt_cli;
