// kavach bg — clap subcommand definitions for the CC 2.1.152+ /bg bridge.
// Wraps Claude Code's /bg primitive so kavach can spawn background sessions
// keyed to a roadmap unit + persist active-bg state for cross-turn pickup.
// SOURCE: roadmap.unit.kavach-bg-session · code.claude.com/docs/en/changelog 2.1.152.
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub(crate) struct BgArgs {
    #[command(subcommand)]
    pub action: BgAction,
}

#[derive(Debug, Subcommand)]
pub(crate) enum BgAction {
    /// Declare a background-session intent. Prints `/bg <task>` for the agent
    /// to paste into a CC session — kavach records the bg row in kavach-db
    /// so subsequent turns can detect "bg in flight" and yield (stop-gate
    /// already short-circuits on `input.background_tasks`).
    Start {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Roadmap key the bg session will work on (e.g. roadmap.unit.<x>).
        #[arg(long)]
        task: String,
        /// Worktree isolation: "none" (default — shared FS), "worktree" (separate copy).
        /// SOURCE: code.claude.com/docs/en/changelog 2.1.152 `worktree.bgIsolation`.
        #[arg(long, default_value = "none")]
        isolation: String,
    },
    /// List active bg sessions for a project.
    Status {
        #[arg(long)]
        project: String,
    },
    /// Clear a bg-session row (after task completion or manual abort).
    Stop {
        #[arg(long)]
        project: String,
        #[arg(long)]
        task: String,
    },
}
