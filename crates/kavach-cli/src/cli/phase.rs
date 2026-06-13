// ARCH: PhaseCLI — CLI interface for SDLC phase management
// PATTERN: phase_gate | SCOPE: cli | CAP: AP | SEARCHED: 2026-04
// Per Stanford Meta-Harness: harness sequences enforcement for depth over breadth.

use clap::Subcommand;

#[derive(Subcommand)]
#[command(
    about = "SDLC phase gates (PLAN → IMPLEMENT → TEST → HARDEN)",
    long_about = "Enforces depth-over-breadth: one file per iteration during IMPLEMENT, \
Definition-of-Done checks before phase advance.\n\n\
Phases: PLAN, IMPLEMENT, TEST, HARDEN.",
    after_help = "EXAMPLES:\n  kavach phase status\n  \
kavach phase iteration-start crates/foo/src/lib.rs\n  \
kavach phase iteration-done\n  \
kavach phase iteration-list\n\nWHEN: Every edit during IMPLEMENT — lock one file, then release."
)]
pub(crate) enum PhaseAction {
    /// Show current phase and iteration status
    Status,
    /// Advance to the next phase (if Definition of Done is met)
    Advance,
    /// Set the current phase (admin override)
    Set {
        /// Phase name: PLAN, IMPLEMENT, TEST, or HARDEN
        phase: String,
    },
    /// Start an iteration on a specific file
    IterationStart {
        /// File path to work on (must match the file you will edit this turn)
        file: String,
    },
    /// Mark the current iteration file as done
    IterationDone,
    /// List files completed in the current phase
    IterationList,
    /// Set project tier (refactor, feature, platform)
    TierSet {
        /// Target tier: refactor, feature, or platform
        tier: String,
        /// Project slug
        #[arg(long)]
        project: String,
        /// Change reason
        #[arg(long)]
        reason: String,
        /// Allow downgrade (otherwise promotion-only)
        #[arg(long)]
        override_flag: bool,
    },
    /// Start spike mode (bypass spec gates)
    SpikeStart {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Duration in hours
        #[arg(long)]
        hours: u32,
        /// Spike reason
        #[arg(long)]
        reason: String,
    },
    /// End spike mode
    SpikeEnd {
        /// Project slug
        #[arg(long)]
        project: String,
    },
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use crate::cli::Cli;

    #[test]
    fn phase_subcommands_are_valid() {
        Cli::command().debug_assert();
    }
}
