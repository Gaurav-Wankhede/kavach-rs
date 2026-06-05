use kavach_types::HookInput;

/// `MessageDisplay` gate (`CC` 2.1.152+): runs just before CC renders an assistant
/// message to the user, with the chance to transform what is shown.
///
/// Kavach's conservative default is **pass-through** — display is a fail-open
/// surface, so an unproven transform must never swallow or mangle the model's
/// output. Emitting silent (no `additional_context`, no replacement) leaves the
/// message exactly as written. The wiring exists so future, evidence-backed
/// redaction (e.g. stripping a leaked secret from rendered output) has a home
/// without re-plumbing the dispatch table.
///
/// SOURCE: code.claude.com/docs/en/changelog v2.1.152 (`MessageDisplay` hook event).
pub(crate) fn run(_input: &HookInput) {
    // Pass-through: do not transform. Stay silent so CC renders the message verbatim.
    drop(kavach_hook::exit_silent());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_display_passes_through() {
        let input = HookInput {
            hook_event_name: "MessageDisplay".into(),
            last_assistant_message: "hello world".into(),
            ..Default::default()
        };
        // Pass-through must not panic and must not block.
        run(&input);
    }
}
