use kavach_types::HookInput;

use crate::gates::rules_manifest::active_rules;

/// `InstructionsLoaded` gate: track which CLAUDE.md files are loaded.
/// Observability only — no blocking capability.
pub(crate) fn run(input: &HookInput) {
    let file_path = &input.file_path;
    let load_reason = &input.load_reason;

    if file_path.is_empty() {
        drop(kavach_hook::exit_silent());
        return;
    }

    let rules = active_rules();
    let rules_count = rules.len().to_string();
    let context = kavach_hook::context_block(
        "INSTRUCTIONS_LOADED",
        &[
            ("file", file_path),
            ("reason", load_reason),
            ("active_rules", &rules_count),
        ],
    );
    drop(kavach_hook::exit_notification_context(&context));
}
