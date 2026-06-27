// TIME: O(1) | SPACE: O(1)
//! exec_prompt write-boundary gate — a NEW roadmap card is born `todo` and is
//! unservable (`db next-prompt` exit 1) without a work order, so reject it here.
const ERR: &str = "[EXEC_PROMPT_GATE] a new roadmap card is a todo and is unservable \
without an exec_prompt. Author the seven-block work order \
(ROLE·TASK·FILES·CONSTRAINTS·VERIFY·DONE WHEN·ON FAILURE) and pass --exec-prompt. \
See the exec-prompt skill. Bypass (migrations only): KAVACH_EXEC_PROMPT_BYPASS=1.";
/// Reason string when a new roadmap write must carry a non-blank exec_prompt;
/// `None` when the write is allowed. Pure — env read is the caller's job.
#[must_use]
pub(super) fn blocked(
    category: &str,
    is_new: bool,
    exec_prompt: Option<&str>,
) -> Option<&'static str> {
    if category != "roadmap" || !is_new {
        return None;
    }
    if exec_prompt.is_some_and(|p| !p.trim().is_empty()) {
        return None;
    }
    Some(ERR)
}
#[cfg(test)]
#[path = "exec_prompt_gate_test.rs"]
#[path = "exec_prompt_gate_test.rs"]
mod tests;
