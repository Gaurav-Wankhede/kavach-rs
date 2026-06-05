//! Write-bypass detection subsystem: catch Bash commands that mutate files or
//! the database outside the Write/Edit hooks. Decomposed by responsibility:
//! `segment` (shell command-position helpers), `redirect` (`>`/`>>` grammar),
//! `tool_write` (file-writing CLIs), `psql` (bare-psql block), `sqlx_migrate`
//! (production-migration RCA gate), and `detect` (the `is_write_bypass`
//! aggregate). The parent `pre_tool_bash` module consumes the three public
//! entry points re-exported here.

mod detect;
mod psql;
mod redirect;
mod segment;
mod source_target;
mod sqlx_migrate;
mod tool_write;

pub(super) use detect::is_write_bypass;
pub(super) use psql::check_psql_blocked;
pub(super) use source_target::targets_tracked_source;
pub(super) use sqlx_migrate::check_sqlx_migrate_requires_rca;
