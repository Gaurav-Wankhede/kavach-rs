// kavach goal — bridge to CC 2.1.139+ /goal primitive for cross-turn loops.
// SOURCE: roadmap.unit.kavach-goal-bridge.
mod compile;
mod loop_yaml;
mod reconcile;
mod start;
mod status;
mod stop;
mod types;

pub(crate) use types::{GoalAction, GoalArgs};

pub(crate) fn run(args: GoalArgs) -> i32 {
    match args.action {
        GoalAction::Start {
            project,
            condition,
            evaluator,
            roadmap_key,
            oracle_check,
        } => start::run(
            &project,
            &condition,
            &evaluator,
            roadmap_key.as_deref(),
            oracle_check.as_deref(),
        ),
        GoalAction::Status { project } => status::run(&project),
        GoalAction::Stop { project, condition } => stop::run(&project, &condition),
        GoalAction::Compile { goal_id } => compile::run(&goal_id),
        GoalAction::Reconcile {
            goal_id,
            oracle_result,
        } => reconcile::run(&goal_id, &oracle_result),
    }
}
