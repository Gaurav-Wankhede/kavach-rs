use super::*;

#[test]
fn test_notification_default() {
    let input = HookInput {
        message: "test notification".into(),
        notification_type: "info".into(),
        ..Default::default()
    };
    run(&input);
}

#[test]
fn bell_on_permission_and_idle_only() {
    // Attention-needing types ring the bell.
    assert_eq!(terminal_sequence_for("permission", ""), "\x07");
    assert_eq!(terminal_sequence_for("Idle", ""), "\x07");
    assert_eq!(terminal_sequence_for("error", ""), "\x07");
    // Informational types stay silent.
    assert_eq!(terminal_sequence_for("info", ""), "");
    // Empty type falls back to the message text.
    assert_eq!(
        terminal_sequence_for("", "Claude needs your permission"),
        "\x07"
    );
    assert_eq!(terminal_sequence_for("", "task done"), "");
}

#[test]
fn agent_completed_injects_context() {
    let input = HookInput {
        message: "agent work done".into(),
        notification_type: "agent_completed".into(),
        ..Default::default()
    };
    run(&input);
}

#[test]
fn agent_needs_input_injects_context() {
    let input = HookInput {
        message: "agent waiting".into(),
        notification_type: "agent_needs_input".into(),
        ..Default::default()
    };
    run(&input);
}
