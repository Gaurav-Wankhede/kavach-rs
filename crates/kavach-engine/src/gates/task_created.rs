//! `TaskCreated` gate — logs task creation events for session tracking.
//! Wired to `TaskCreate` tool `PostToolUse` event.

use std::io::Write;

use kavach_types::HookInput;

use crate::error::EngineError;

#[expect(clippy::unnecessary_wraps, reason = "uniform gate dispatch")]
pub(crate) fn run(input: &HookInput) -> Result<(), EngineError> {
    let task_subject = &input.task_subject;
    let task_id = &input.task_id;

    let mut session = kavach_session::get_or_create_session();
    session.tasks_created = session.tasks_created.saturating_add(1);

    if !task_subject.is_empty() {
        session.add_case_fact(&format!("task created: {task_subject}"));
    }
    if let Err(e) = session.save() {
        writeln!(
            std::io::stderr().lock(),
            "kavach: task_created save failed: {e}"
        )
        .ok();
    }

    let created_str = session.tasks_created.to_string();
    let context = kavach_hook::context_block(
        "TASK_CREATED",
        &[
            ("task_id", task_id.as_str()),
            ("subject", task_subject.as_str()),
            ("created_count", &created_str),
        ],
    );
    drop(kavach_hook::exit_notification_context(&context));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_created_empty_subject() {
        let input = HookInput::default();
        assert!(run(&input).is_ok());
    }

    #[test]
    fn test_task_created_with_subject() {
        let input = HookInput {
            task_id: "task-42".into(),
            task_subject: "implement auth gate".into(),
            ..Default::default()
        };
        assert!(run(&input).is_ok());
    }
}
