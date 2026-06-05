//! Bulk-checkbox detection: flag a single write that checks 10+ plan items at
//! once (forbidden — plan checkboxes must be verified individually vs code).

/// Detect bulk checkbox changes in plan/markdown files.
/// Returns Some(warning) if content has 10+ checked items (`- [x]`)
/// being added in a single write to a plan-like file.
#[must_use]
pub(crate) fn detect_bulk_checkbox(path: &str, content: &str, new_str: &str) -> Option<String> {
    if !std::path::Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("md"))
    {
        return None;
    }
    let text = if content.is_empty() { new_str } else { content };
    let checked_count = text
        .matches("- [x]")
        .count()
        .saturating_add(text.matches("- [X]").count());
    if checked_count >= 10 {
        return Some(format!(
            "[BULK_CHECKBOX_WARNING] {checked_count} checked items (`- [x]`) detected \
             in a single write to {path}. Plan checkboxes must be verified \
             individually against code before checking. Bulk-checking is forbidden."
        ));
    }
    None
}
