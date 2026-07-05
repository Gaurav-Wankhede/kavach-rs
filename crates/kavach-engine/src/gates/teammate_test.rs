use super::*;

#[test]
fn test_run_idle() {
    let input = HookInput {
        teammate_name: "researcher".into(),
        ..Default::default()
    };
    assert!(run_idle(&input).is_ok());
}

#[test]
fn test_task_completed_with_subject() {
    let input = HookInput {
        task_id: "task-123".into(),
        task_subject: "implement auth".into(),
        ..Default::default()
    };
    assert!(run_task_completed(&input).is_ok());
}
