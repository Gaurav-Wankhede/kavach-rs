/// Nudge a roadmap write that omits a (non-blank) `exec_prompt`; None otherwise.
#[must_use]
pub(crate) fn advise(category: &str, exec_prompt: Option<&str>) -> Option<String> {
    if category != "roadmap" {
        return None;
    }
    if exec_prompt.is_some_and(|p| !p.trim().is_empty()) {
        return None;
    }
    Some(
        "[EXEC_PROMPT_P1] roadmap card has no --exec-prompt. Author the seven-block \
         executor work order (ROLE·TASK·FILES·CONSTRAINTS·VERIFY·DONE WHEN·ON FAILURE) \
         so `kavach db next-prompt` can serve it to Haiku/Composer 2.5. See the \
         exec-prompt skill; an empty prompt is rejected at serve time (exit 1)."
            .to_owned(),
    )
}
#[cfg(test)]
#[path = "exec_prompt_advice_test.rs"]
#[path = "exec_prompt_advice_test.rs"]
mod tests;
