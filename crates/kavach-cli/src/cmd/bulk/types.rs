// kavach bulk — clap subcommand definitions (declarative only; logic in start/status/close).
// SOURCE: roadmap.unit.kavach-bulk-mode acceptance #6.
use clap::{Args, Subcommand};

pub(crate) const DEFAULT_TTL_SECONDS: i64 = 3600;

#[derive(Debug, Args)]
#[command(
    about = "Bulk-mode: one [RCA] binds N edits in a mechanical sweep",
    long_about = "Opens a manifest that authorizes many similar edits under one root-cause \
analysis file. Gates treat the manifest as a single scoped exception instead of N ad-hoc \
writes.\n\nWHEN: Large lint-class sweeps (rename, import fix, pattern migration) where \
each file change shares the same RCA.",
    after_help = "EXAMPLES:\n  \
kavach bulk start --sweep-id lint-unused --project P --rca-file docs/rca.md \\\n    \
--scope-glob 'src/**/*.rs' --lint-class clippy --fix-strategy auto --blast-estimate 40\n  \
kavach bulk status --project P\n  \
kavach bulk close --sweep-id lint-unused [--reason closed|expired]"
)]
pub(crate) struct BulkArgs {
    #[command(subcommand)]
    pub action: BulkAction,
}

#[derive(Debug, Subcommand)]
pub(crate) enum BulkAction {
    /// Open a manifest binding N edits to ONE shared [RCA].
    Start {
        /// Unique sweep id (reuse blocked while manifest is open).
        #[arg(long)]
        sweep_id: String,
        /// Project slug the sweep applies to.
        #[arg(long)]
        project: String,
        /// Path to the RCA markdown file (absolute or repo-relative).
        #[arg(long)]
        rca_file: String,
        /// Glob of files in scope (e.g. `src/**/*.rs`).
        #[arg(long)]
        scope_glob: String,
        /// Lint or defect class being swept (e.g. clippy, `dead_code`).
        #[arg(long)]
        lint_class: String,
        /// Fix strategy label (e.g. auto, manual-review).
        #[arg(long)]
        fix_strategy: String,
        /// Estimated number of files touched (blast-radius hint for gates).
        #[arg(long)]
        blast_estimate: i64,
        /// Manifest TTL in seconds (default 3600); auto-expires if not closed.
        #[arg(long, default_value_t = DEFAULT_TTL_SECONDS)]
        ttl_seconds: i64,
        /// Who approved the sweep (audit field, default \"user\").
        #[arg(long, default_value = "user")]
        approved_by: String,
    },
    /// List active manifests for a project.
    Status {
        /// Project slug whose open manifests to list.
        #[arg(long)]
        project: String,
    },
    /// Close a manifest. --reason {closed|expired}.
    Close {
        /// Sweep id to close (from `bulk start` or `bulk status`).
        #[arg(long)]
        sweep_id: String,
        /// Close reason: closed (success) or expired (TTL).
        #[arg(long, default_value = "closed")]
        reason: String,
    },
}
