// ARCH: see kavach db get --category decision --key arch.decision.fourteen_prefix_const_table

use kavach_types::HookInput;
use std::sync::LazyLock;

use crate::error::EngineError;

static INTENT_REGEX: LazyLock<Option<regex::Regex>> = LazyLock::new(|| {
    regex::RegexBuilder::new(
        r"build (a |the |new |feature|module|service)|add (a |new |feature|page|endpoint|component)|implement |create (a |the |new )(feature|page|component|endpoint|service|module)|draft (a )?spec|decompose|plan( |ning) (this|the|a) (build|feature|project)|new feature|next unit|what should i build|write the spec|write a spec",
    )
    .case_insensitive(true)
    .build()
    .ok()
});

#[expect(
    clippy::unnecessary_wraps,
    reason = "signature fixed by run_gate dispatch table: every gate handler returns Result<(), EngineError>"
)]
pub(crate) fn run(input: &HookInput) -> Result<(), EngineError> {
    let prompt = input.get_string("prompt");
    if prompt.is_empty() {
        return Ok(());
    }

    let Some(regex) = INTENT_REGEX.as_ref() else {
        return Ok(());
    };
    if !regex.is_match(prompt) {
        return Ok(());
    }

    emit_six_file_context();
    Ok(())
}

fn emit_six_file_context() {
    let context = r"[SIX_FILE_GATE]
Planning/feature intent detected. BEFORE Skill `writing-plans`, BEFORE any Edit/Write on new feature surface, you MUST:

  1. Invoke Skill `six-file-context` (loads kavach-db read protocol)
  2. Run the witness chain:
     kavach db get --project <slug> --category app_spec --key spec.overview
     kavach db get --project <slug> --category architecture --key-prefix arch.invariant
     kavach db get --project <slug> --category roadmap --key-prefix roadmap.unit --full
  3. If any witness returns empty, route to Agent `spec-author` (read-only) to draft missing rows; parent writes via `kavach db write`.
  4. Check `spec.scope.out.*` BEFORE adding any feature; refuse if a matching out-row exists.

Reference: ~/.claude/CLAUDE.md §15 — Six-File Context Protocol.";

    let json = format!(
        r#"{{"hookSpecificOutput":{{"hookEventName":"UserPromptSubmit","additionalContext":{context:?}}}}}"#
    );
    drop(kavach_hook::exit_prompt_context(&json));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_match_build() {
        let input =
            serde_json::from_str::<HookInput>(r#"{"prompt": "build a new feature for auth"}"#)
                .unwrap();
        assert!(run(&input).is_ok());
    }

    #[test]
    fn test_intent_match_implement() {
        let input =
            serde_json::from_str::<HookInput>(r#"{"prompt": "implement the notification system"}"#)
                .unwrap();
        assert!(run(&input).is_ok());
    }

    #[test]
    fn test_intent_no_match() {
        let input =
            serde_json::from_str::<HookInput>(r#"{"prompt": "show me the current code"}"#).unwrap();
        assert!(run(&input).is_ok());
    }

    #[test]
    fn test_intent_case_insensitive() {
        let input = serde_json::from_str::<HookInput>(r#"{"prompt": "BUILD a module"}"#).unwrap();
        assert!(run(&input).is_ok());
    }
}
