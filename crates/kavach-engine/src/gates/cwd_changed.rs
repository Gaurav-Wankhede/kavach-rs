//! `CwdChanged` gate — resets session state when working directory changes.
//! Prevents cross-project contamination of `research_done`, `intent_risk`, skills.

use std::io::Write;

use kavach_types::HookInput;

use crate::error::EngineError;

#[expect(
    clippy::unnecessary_wraps,
    reason = "signature fixed by run_gate dispatch table: every gate handler returns Result<(), EngineError>"
)]
pub(crate) fn run(input: &HookInput) -> Result<(), EngineError> {
    let old_cwd = input.get_string("old_cwd");
    let new_cwd = input.get_string("new_cwd");

    if old_cwd.is_empty() || new_cwd.is_empty() || old_cwd == new_cwd {
        drop(kavach_hook::exit_silent());
        return Ok(());
    }

    let mut session = kavach_session::get_or_create_session();
    session.research_done = false;
    session.research_topic.clear();
    session.research_topics.clear();
    session.intent_risk = String::from("medium");
    session.invoked_skills.clear();
    session.required_skills.clear();
    session.add_case_fact(&format!(
        "cwd changed: {old_cwd} → {new_cwd} — session state reset"
    ));
    if let Err(e) = session.save() {
        writeln!(
            std::io::stderr().lock(),
            "kavach: cwd_changed save failed: {e}"
        )
        .ok();
    }

    let context = kavach_hook::context_block(
        "CWD_CHANGED",
        &[
            ("old_cwd", old_cwd),
            ("new_cwd", new_cwd),
            ("status", "session_state_reset"),
        ],
    );
    drop(kavach_hook::exit_notification_context(&context));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_cwd_is_noop() {
        let input = HookInput::default();
        assert!(run(&input).is_ok());
    }

    #[test]
    fn test_empty_cwd_is_noop() {
        let input = HookInput {
            cwd: String::new(),
            ..Default::default()
        };
        assert!(run(&input).is_ok());
    }
}
