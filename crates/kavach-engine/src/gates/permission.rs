use kavach_types::HookInput;

/// Permission gate: handle `PermissionRequest` events.
/// Decides whether to allow/deny tool permission requests.
pub(crate) fn run(input: &HookInput) {
    let tool_name = &input.tool_name;

    // Check if tool is in the auto-allow list
    if kavach_config::is_auto_allowed_tool(tool_name) {
        drop(kavach_hook::exit_permission_allow(&format!(
            "auto-allowed: {tool_name}"
        )));
        return;
    }

    // Check if command is blocked
    if tool_name == "Bash" {
        let command = input.get_string("command");
        if kavach_config::is_blocked_command(command) {
            drop(kavach_hook::exit_permission_deny(&format!(
                "PERMISSION DENIED: destructive command: `{command}`. \
                 FIX: Use targeted, non-destructive alternatives. \
                 See pre-tool bash gate for specific safe replacements."
            )));
            return;
        }
    }

    // Check write path permissions
    let file_path = input.get_string("file_path");
    if !file_path.is_empty() && kavach_config::is_blocked_write_path(file_path) {
        drop(kavach_hook::exit_permission_deny(&format!(
            "PERMISSION DENIED: system path write: {file_path}. \
             Paths under /etc/, /usr/, /bin/, /.ssh/, /.aws/ are protected. \
             FIX: Write to project directory or ~/.local/ for user files."
        )));
        return;
    }

    // Default: allow with context
    drop(kavach_hook::exit_permission_allow(&format!(
        "permitted: {tool_name}"
    )));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_default() {
        let input = HookInput {
            tool_name: "Read".into(),
            ..Default::default()
        };
        run(&input);
    }
}
