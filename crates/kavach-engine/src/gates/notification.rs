use kavach_types::HookInput;

/// Notification gate: handle Notification events.
/// Injects context based on notification type.
pub(crate) fn run(input: &HookInput) {
    let message = &input.message;
    let notification_type = &input.notification_type;

    // Do NOT increment_turn() for notifications. They are informational,
    // not tool interactions. Incrementing would defeat failure staleness:
    // a notification between failure and stop check would let Claude stop
    // despite unresolved failures (has_recent_failure uses turn equality).

    let msg_preview: String = message.chars().take(200).collect();
    let context = kavach_hook::context_block(
        "NOTIFICATION",
        &[("type", notification_type), ("message", &msg_preview)],
    );

    // CC 2.1.141: ring the terminal bell on attention-needing notifications
    // (permission stalls, idle prompts, errors) so a backgrounded session is
    // noticed; stay silent on purely informational ones.
    drop(kavach_hook::exit_notification_with_sequence(
        &context,
        terminal_sequence_for(notification_type, message),
    ));
}

/// Pick a terminal escape sequence by notification type. Bell (`\x07`) on the
/// attention-needing classes CC emits — permission requests and idle waits —
/// matched case-insensitively on the type, with a message-substring fallback for
/// CC builds that leave `notification_type` empty. Empty string = emit nothing.
fn terminal_sequence_for(notification_type: &str, message: &str) -> &'static str {
    let ty = notification_type.to_ascii_lowercase();
    let attention = ty.contains("permission")
        || ty.contains("idle")
        || ty.contains("waiting")
        || ty.contains("error");
    if attention {
        return "\x07";
    }
    // Fallback: some CC builds send an empty type with the text in `message`.
    let msg = message.to_ascii_lowercase();
    if ty.is_empty() && (msg.contains("permission") || msg.contains("waiting for your input")) {
        return "\x07";
    }
    ""
}

#[cfg(test)]
mod tests {
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
}
