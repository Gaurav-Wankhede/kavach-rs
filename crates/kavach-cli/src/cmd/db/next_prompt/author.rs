// TIME: O(n) in card text | SPACE: O(n)
//! Build the Haiku authoring prompt for a promptless top card — kavach fills the
//! exec_prompt in-place (no skip) so the main LLM never reasons about the gap.
/// Compose the seven-block authoring instruction from the card's own fields.
#[must_use]
pub(super) fn authoring_prompt(project: &str, key: &str, title: &str, content: &str) -> String {
    format!(
        "You are authoring a self-contained executor work order for a roadmap card. \
The executor (Haiku/Composer) runs it verbatim with NO conversation context and CANNOT ask. \
Output ONLY the prompt — seven labeled blocks, no preamble:\n\
ROLE · TASK · FILES · CONSTRAINTS · VERIFY · DONE WHEN · ON FAILURE.\n\n\
Project: {project}\nCard key: {key}\nTitle: {title}\n\nCard detail:\n{content}"
    )
}
#[cfg(test)]
#[path = "author_test.rs"]
mod tests;
