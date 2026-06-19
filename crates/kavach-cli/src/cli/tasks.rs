use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum TasksAction {
    /// Audit Claude Code `TaskCreate` JSON store and infer per-task project holdership.
    /// Reads ~/.claude/tasks/<user>/<id>.json files and prints a table mapping each
    /// task to its likely project via keyword matching.
    Audit {
        /// Override the Claude Code user dir under ~/.claude/tasks/ (auto-detected if omitted)
        #[arg(long)]
        user: Option<String>,
    },
}
