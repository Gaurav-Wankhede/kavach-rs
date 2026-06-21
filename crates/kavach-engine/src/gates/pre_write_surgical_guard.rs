// Karpathy Principle 3: "Surgical Changes"
// Advisory-only gate: diff size + per-turn file scope tracking.
// Never blocks — injects [SURGICAL_WARNING] or [SCOPE_WARNING] context.

const DIFF_LINE_THRESHOLD: usize = 50;
const FILES_PER_TURN_THRESHOLD: usize = 5;

/// Diff-size advisory: warn when a single Edit has > 50 changed lines.
/// Returns Some([`SURGICAL_WARNING`]) when threshold exceeded, None otherwise.
pub(crate) fn diff_advisory(tool_name: &str, new_string: &str) -> Option<String> {
    if tool_name != "Edit" {
        return None;
    }
    let diff_lines = new_string.lines().count();
    if diff_lines <= DIFF_LINE_THRESHOLD {
        return None;
    }
    Some(format!(
        "[SURGICAL_WARNING]\n\
         diff_lines: {diff_lines}\n\
         action: Large edit detected. Ensure every changed line traces to the user's request.\n"
    ))
}

/// Scope advisory: warn when > 5 distinct files have been modified in the current turn.
/// `files_this_turn` is the list from `session.files_modified_this_turn`.
pub(crate) fn scope_advisory(files_this_turn: &[String]) -> Option<String> {
    if files_this_turn.len() <= FILES_PER_TURN_THRESHOLD {
        return None;
    }
    Some(format!(
        "[SCOPE_WARNING]\n\
         files_this_turn: {}\n\
         action: Many files modified — SPLIT this into focused single-concern passes now.\n",
        files_this_turn.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_none_for_write_tool() {
        let big = "line\n".repeat(100);
        assert!(diff_advisory("Write", &big).is_none());
    }

    #[test]
    fn should_return_none_for_small_edit() {
        let small = "line\n".repeat(10);
        assert!(diff_advisory("Edit", &small).is_none());
    }

    #[test]
    fn should_warn_for_large_edit() {
        let big = "line\n".repeat(60);
        let result = diff_advisory("Edit", &big);
        assert!(result.is_some());
        let s = result.unwrap_or_default();
        assert!(s.contains("[SURGICAL_WARNING]"));
        assert!(s.contains("60"));
    }

    #[test]
    fn should_return_none_for_few_files() {
        let files: Vec<String> = (0..4).map(|i| format!("file{i}.rs")).collect();
        assert!(scope_advisory(&files).is_none());
    }

    #[test]
    fn should_warn_for_many_files() {
        let files: Vec<String> = (0..7).map(|i| format!("file{i}.rs")).collect();
        let result = scope_advisory(&files);
        assert!(result.is_some());
        let s = result.unwrap_or_default();
        assert!(s.contains("[SCOPE_WARNING]"));
        assert!(s.contains('7'));
    }

    #[test]
    fn should_warn_at_exactly_threshold_plus_one() {
        let files: Vec<String> = (0..6).map(|i| format!("file{i}.rs")).collect();
        assert!(scope_advisory(&files).is_some());
    }

    #[test]
    fn should_not_warn_at_exactly_threshold() {
        let files: Vec<String> = (0..5).map(|i| format!("file{i}.rs")).collect();
        assert!(scope_advisory(&files).is_none());
    }
}
