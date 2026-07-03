use kavach_types::HookInput;

/// Notification gate: handle Notification events.
/// Injects context based on notification type.
/// SOURCE: https://raw.githubusercontent.com/anthropics/claude-code/main/CHANGELOG.md
pub(crate) fn run(input: &HookInput) {
    let message = &input.message;
    let notification_type = &input.notification_type;

    // Do NOT increment_turn() for notifications. They are informational,
    // not tool interactions. Incrementing would defeat failure staleness:
    // a notification between failure and stop check would let Claude stop
    // despite unresolved failures (has_recent_failure uses turn equality).

    let msg_preview: String = message.chars().take(200).collect();
    let ty_lower = notification_type.to_ascii_lowercase();
    let mut full_context = kavach_hook::context_block(
        "NOTIFICATION",
        &[("type", notification_type), ("message", &msg_preview)],
    );

    if ty_lower == "agent_completed" {
        let completed_ctx = kavach_hook::context_block(
            "AGENT_COMPLETED",
            &[("guidance", "run three-witness verify: artifact exists, diff landed, build passes")],
        );
        full_context.push_str(&completed_ctx);
    } else if ty_lower == "agent_needs_input" {
        let input_ctx = kavach_hook::context_block(
            "AGENT_NEEDS_INPUT",
            &[("guidance", "prefer querying kavach DB or reading code over asking")],
        );
        full_context.push_str(&input_ctx);
    }

    // CC 2.1.141: ring the terminal bell on attention-needing notifications
    // (permission stalls, idle prompts, errors) so a backgrounded session is
    // noticed; stay silent on purely informational ones.
    drop(kavach_hook::exit_notification_with_sequence(
        &full_context,
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
#[path = "notification_test.rs"]
mod tests;
