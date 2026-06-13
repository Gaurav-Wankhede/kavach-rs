// ARCH: AutonomousLoopCLI — harness engineering loop-until-complete
// PATTERN: pev_loop (Plan-Execute-Verify) | SCOPE: session | CAP: AP | SEARCHED: 2026-05
// SOURCE: martinfowler.com/articles/harness-engineering.html
use clap::Subcommand;

#[derive(Subcommand)]
#[command(
    about = "Autonomous execution loop (harness engineering — loop until target met)",
    long_about = "Runs PEV iterations until a target condition holds or max iterations \
is reached. Targets: phase:TEST, kanban:empty, goal.",
    after_help = "EXAMPLES:\n  kavach loop start --target 'kanban:empty' --max 50\n  \
kavach loop status\n  kavach loop stop\n\nWHEN: Long autonomous sessions with a clear stop condition."
)]
pub(crate) enum LoopAction {
    /// Start an autonomous execution loop with a target condition
    Start {
        /// Target condition: \"phase:TEST\", \"kanban:empty\", \"goal\"
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
