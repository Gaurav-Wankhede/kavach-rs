use kavach_types::HookInput;

use crate::error::EngineError;

/// Handle `TeammateIdle` event — enforce quality gates before teammate stops.
/// Per CC 2.1: "Use this to enforce quality gates before a teammate stops working."
#[expect(
    clippy::unnecessary_wraps,
    reason = "hook protocol contract requires Result return type"
)]
pub(crate) fn run_idle(input: &HookInput) -> Result<(), EngineError> {
    let teammate_name = &input.teammate_name;
    let mut session = kavach_session::get_or_create_session();

    // Track teammate going idle
    session.track_teammate_stop(teammate_name);

    // Quality gate: block idle if recent failure — force fix before stopping
    if session.has_recent_failure() {
        let tool = session.last_failure_tool.clone();
        let reason = format!("[TEAM_POLICY] {tool} failed -> {teammate_name} fix before idle -> retry.");
        drop(kavach_hook::exit_stop_block(&reason));
        return Ok(());
    }

    let context = kavach_hook::context_block("TEAMMATE_IDLE", &[("teammate", teammate_name)]);
    drop(kavach_hook::exit_notification_context(&context));
    Ok(())
}

/// Handle `TaskCompleted` event — enforce completion criteria, track task details.
/// Per CC 2.1: "Use this to enforce completion criteria before a task can close."
#[expect(
    clippy::unnecessary_wraps,
    reason = "hook protocol contract requires Result return type"
)]
pub(crate) fn run_task_completed(input: &HookInput) -> Result<(), EngineError> {
    let task_id = &input.task_id;
    let task_subject = &input.task_subject;

    let mut session = kavach_session::get_or_create_session();
    session.tasks_completed = session.tasks_completed.saturating_add(1);
    if !task_subject.is_empty() {
        session.set_task(task_subject, "completed");
    }
    session.save().ok();

    let completed_str = session.tasks_completed.to_string();
    // A completed task may unblock dependents in the DAG. Emit a wake advisory
    // so the parent re-ticks the DagScheduler (event-driven, no polling loop).
    // Tag + "DagScheduler" mechanic frozen; the imperative is research-refreshed.
    let dag_wake = crate::gates::directive_cache::dyn_directive(
        "teammate.dag-wake",
        "[DAG_WAKE] re-tick DagScheduler — task closed may unblock dependents",
    );
    let context = kavach_hook::context_block(
        "TASK_COMPLETED",
        &[
            ("id", task_id),
            ("subject", task_subject),
            ("n", &completed_str),
            ("advisory", &dag_wake),
        ],
    );
    drop(kavach_hook::exit_notification_context(&context));
    Ok(())
}

#[cfg(test)]
#[path = "teammate_test.rs"]
mod tests;
