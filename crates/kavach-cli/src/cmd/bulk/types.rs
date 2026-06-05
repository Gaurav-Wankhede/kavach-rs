// kavach bulk — clap subcommand definitions (declarative only; logic in start/status/close).
// SOURCE: roadmap.unit.kavach-bulk-mode acceptance #6.
use clap::{Args, Subcommand};

pub(crate) const DEFAULT_TTL_SECONDS: i64 = 3600;

#[derive(Debug, Args)]
pub(crate) struct BulkArgs {
    #[command(subcommand)]
    pub action: BulkAction,
}

#[derive(Debug, Subcommand)]
pub(crate) enum BulkAction {
    /// Open a manifest binding N edits to ONE shared [RCA].
    Start {
        #[arg(long)]
        sweep_id: String,
        #[arg(long)]
        project: String,
        #[arg(long)]
        rca_file: String,
        #[arg(long)]
        scope_glob: String,
        #[arg(long)]
        lint_class: String,
        #[arg(long)]
        fix_strategy: String,
        #[arg(long)]
        blast_estimate: i64,
        #[arg(long, default_value_t = DEFAULT_TTL_SECONDS)]
        ttl_seconds: i64,
        #[arg(long, default_value = "user")]
        approved_by: String,
    },
    /// List active manifests for a project.
    Status {
        #[arg(long)]
        project: String,
    },
    /// Close a manifest. --reason {closed|expired}.
    Close {
        #[arg(long)]
        sweep_id: String,
        #[arg(long, default_value = "closed")]
        reason: String,
    },
}
