// kavach goal — clap subcommand definitions for the CC 2.1.139+ /goal bridge.
// Wraps Claude Code's built-in /goal primitive so kavach can declare goal
// conditions per project + emit /goal-formatted text via additionalContext.
// SOURCE: roadmap.unit.kavach-goal-bridge · help.apiyi.com/en/claude-code-goal-mode-keep-working-until-done-guide-en.html.
use clap::{Args, Subcommand};

pub(crate) const DEFAULT_EVALUATOR: &str = "haiku";

#[derive(Debug, Args)]
pub(crate) struct GoalArgs {
    #[command(subcommand)]
    pub action: GoalAction,
}

#[derive(Debug, Subcommand)]
pub(crate) enum GoalAction {
    /// Declare a goal condition. Prints `/goal <condition>` for the agent to
    /// paste into a CC session — kavach records the goal in kavach-db for audit.
    Start {
        #[arg(long)]
        project: String,
        #[arg(long)]
        condition: String,
        #[arg(long, default_value = DEFAULT_EVALUATOR)]
        evaluator: String,
        /// Optional roadmap key the goal closes (e.g. roadmap.unit.strict-lint-zero).
        #[arg(long)]
        roadmap_key: Option<String>,
        /// Optional oracle command whose exit code 0 proves the goal is done.
        /// When set, emits `.kavach/goals/<slug>/loop.yaml` (oracle-gated loop).
        #[arg(long)]
        oracle_check: Option<String>,
    },
    /// List active goals for a project.
    Status {
        #[arg(long)]
        project: String,
    },
    /// Stop tracking the goal (emits `/goal clear` for the agent).
    Stop {
        #[arg(long)]
        project: String,
        #[arg(long)]
        condition: String,
    },
    /// Compile a goal's `.kavach/goals/<id>/loop.yaml` into a Claude Code
    /// Workflow `workflow.js` (the oracle-gated loop runner).
    Compile {
        #[arg(long)]
        goal_id: String,
    },
    /// Apply an oracle verdict to the proof-gated completion flag. The Workflow
    /// loop calls this after each oracle run: `pass` lets the goal close,
    /// anything else keeps the stop gate blocking.
    Reconcile {
        #[arg(long)]
        goal_id: String,
        #[arg(long)]
        oracle_result: String,
    },
}
