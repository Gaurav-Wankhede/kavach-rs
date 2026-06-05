// ARCH: AutonomousLoopCLI — harness engineering loop-until-complete
// PATTERN: pev_loop (Plan-Execute-Verify) | SCOPE: session | CAP: AP | SEARCHED: 2026-05
// SOURCE: martinfowler.com/articles/harness-engineering.html
use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum LoopAction {
    /// Start an autonomous execution loop with a target condition
    Start {
        /// Target condition: "phase:TEST", "kanban:empty", "goal"
        #[arg(long)]
        target: String,
        /// Maximum iterations before forced stop (default: 50)
        #[arg(long, default_value_t = 50)]
        max: i32,
    },
    /// Stop the current autonomous loop
    Stop,
    /// Show current loop status
    Status,
}
