use kavach_types::HookInput;

pub(crate) fn check_edit_staleness(input: &HookInput) -> Option<String> {
    if input.tool_name != "Edit" {
        return None;
    }
    let file_path = input.get_string("file_path");
    let old_string = input.get_string("old_string");
    if file_path.is_empty() || old_string.is_empty() {
        return None;
    }
    let Ok(current) = std::fs::read_to_string(file_path) else {
        return None;
    };
    if current.contains(&old_string) {
        return None;
    }
    Some(format!(
        "[EDIT_STALE] The file `{file_path}` changed since you last read it — \
         `old_string` no longer matches. Re-read the file with Read, then retry \
         the Edit with the current content. Do NOT guess; read first."
    ))
}
