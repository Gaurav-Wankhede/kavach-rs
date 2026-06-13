use kavach_types::HookInput;

/// Run content quality guards: assumptions, hallucinations, completion claims.
pub(crate) fn run_content_quality_checks(content: &str, parts: &mut Vec<String>) {
    if content.is_empty() {
        return;
    }
    if let Some(w) = super::assumption_guard::check_for_assumptions(content) {
        parts.push(w);
    }
    if let Some(w) = super::hallucination_guard::check_for_hallucinations(content) {
        parts.push(w);
    }
    let session = kavach_session::get_or_create_session();
    if let Some(w) = super::completion_guard::check_completion_claim(content, &session) {
        parts.push(w);
    }
    if let Some(w) = super::loophole_guard::check_loophole_interrogation(content) {
        parts.push(w);
    }
}

/// Extract the content that was written. For Write tool, it's "content".
/// For Edit tool, it's "`new_string`". For `NotebookEdit`, it's "`new_source`".
#[must_use]
pub(crate) fn read_written_content(input: &HookInput) -> String {
    let content = input.get_string("content");
    if !content.is_empty() {
        return content.to_owned();
    }
    let new_string = input.get_string("new_string");
    if !new_string.is_empty() {
        return new_string.to_owned();
    }
    let new_source = input.get_string("new_source");
    if !new_source.is_empty() {
        return new_source.to_owned();
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_written_content_write() {
        let input = HookInput {
            tool_input: Some(std::collections::HashMap::from([(
                "content".into(),
                serde_json::json!("fn main() {}"),
            )])),
            ..Default::default()
        };
        assert_eq!(read_written_content(&input), "fn main() {}");
    }

    #[test]
    fn test_read_written_content_edit() {
        let input = HookInput {
            tool_input: Some(std::collections::HashMap::from([(
                "new_string".into(),
                serde_json::json!("updated code"),
            )])),
            ..Default::default()
        };
        assert_eq!(read_written_content(&input), "updated code");
    }

    #[test]
    fn test_read_written_content_empty() {
        let input = HookInput::default();
        assert!(read_written_content(&input).is_empty());
    }
}
