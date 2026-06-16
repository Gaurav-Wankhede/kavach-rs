//! `PreCompact` gate — injects custom instructions context before compaction.

use kavach_types::HookInput;

pub(crate) fn run(input: &HookInput) {
    let ci = &input.custom_instructions;
    if ci.is_empty() {
        drop(kavach_hook::exit_silent());
    } else {
        let context = kavach_hook::context_block(
            "PRE_COMPACT",
            &[("custom_instructions", ci), ("date", &kavach_hook::today_full())],
        );
        let mut session = kavach_session::get_or_create_session();
        session.queue_lifecycle_relay(&context);
        // CC path: notification context. Cursor drops allow output — relay above.
        drop(kavach_hook::exit_notification_context(&context));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_instructions_is_silent() {
        let input = HookInput::default();
        run(&input);
    }
}
