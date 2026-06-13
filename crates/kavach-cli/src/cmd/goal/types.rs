// kavach goal — clap subcommand definitions for the CC 2.1.139+ /goal bridge.
// Wraps Claude Code's built-in /goal primitive so kavach can declare goal
// conditions per project + emit /goal-formatted text via additionalContext.
// SOURCE: roadmap.unit.kavach-goal-bridge · help.apiyi.com/en/claude-code-goal-mode-keep-working-until-done-guide-en.html.
use clap::{Args, Subcommand};

pub(crate) const DEFAULT_EVALUATOR: &str = "haiku";

#[derive(Debug, Args)]
#[command(
    about = "Goal-mode: declare a condition for CC 2.1.139+ /goal cross-turn loops",
    long_about = "Records goal conditions in kavach-db and emits `/goal …` text for Claude \
Code sessions. Optional oracle-gated loops compile to Workflow `workflow.js`.\n\n\
WHEN: Agent must keep working until a verifiable condition holds (kanban empty, tests green, etc.).",
    after_help = "EXAMPLES:\n  \
kavach goal start --project P --condition 'kanban todo lane is empty'\n  \
kavach goal start --project P --condition 'cargo test green' --oracle-check 'cargo nextest run -p foo'\n  \
kavach goal status --project P\n  \
kavach goal stop --project P --condition 'kanban todo lane is empty'\n  \
kavach goal compile --goal-id <slug>\n  \
kavach goal reconcile --goal-id <slug> --oracle-result pass"
)]
pub(crate) struct GoalArgs {
    #[command(subcommand)]
    pub action: GoalAction,
}

#[derive(Debug, Subcommand)]
pub(crate) enum GoalAction {
    /// Declare a goal condition. Prints `/goal <condition>` for the agent to
    /// paste into a CC session — kavach records the goal in kavach-db for audit.
    Start {
        /// Project slug the goal belongs to.
        #[arg(long)]
        project: String,
        /// Natural-language done condition (also the /goal string).
        #[arg(long)]
        condition: String,
        /// Evaluator model tier for goal checks (default haiku).
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
        /// Project slug whose active goals to list.
        #[arg(long)]
        project: String,
    },
    /// Stop tracking the goal (emits `/goal clear` for the agent).
    Stop {
        /// Project slug.
        #[arg(long)]
        project: String,
        /// Condition string matching the row to stop (from `goal status`).
        #[arg(long)]
        condition: String,
    },
    /// Compile a goal's `.kavach/goals/<id>/loop.yaml` into a Claude Code
    /// Workflow `workflow.js` (the oracle-gated loop runner).
    Compile {
        /// Goal id slug (directory under `.kavach/goals/`).
        #[arg(long)]
        goal_id: String,
    },
    /// Apply an oracle verdict to the proof-gated completion flag. The Workflow
    /// loop calls this after each oracle run: `pass` lets the goal close,
    /// anything else keeps the stop gate blocking.
    Reconcile {
        /// Goal id slug (same as `goal compile --goal-id`).
        #[arg(long)]
        goal_id: String,
        /// Oracle verdict: pass | fail | skip (only pass closes the goal).
        #[arg(long)]
        oracle_result: String,
    },
}
