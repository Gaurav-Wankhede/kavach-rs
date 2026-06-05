use kavach_types::HookInput;

/// Handle Read tool pre-check: path blocklist + extension + warn paths.
pub(crate) fn handle_read(input: &HookInput) {
    let file_path = input.get_string("file_path");

    if kavach_config::is_blocked_path(file_path) {
        drop(kavach_hook::exit_pre_tool_deny(&format!(
            "BLOCKED: {file_path} is a system credential/key file. \
             Read from environment variables or a .env file instead — \
             system credential and key files are off-limits."
        )));
        return;
    }

    if kavach_config::is_blocked_extension(file_path) {
        drop(kavach_hook::exit_pre_tool_deny(&format!(
            "BLOCKED: {file_path} has a restricted extension (.pem/.key/.p12/.pfx). \
             Use `openssl x509 -text -noout` via Bash to inspect certificate \
             metadata — reading private key material directly exposes key material."
        )));
        return;
    }

    // Block /proc/self/environ — exposes all process environment values.
    if file_path.contains("/proc/") && file_path.contains("/environ") {
        drop(kavach_hook::exit_pre_tool_deny(
            "BLOCKED: /proc/*/environ exposes all process environment variables including secrets. \
             Use `rg -o '^[A-Z][A-Z0-9_]*' .env | sort` to list variable names only (toolbelt: rg).",
        ));
        return;
    }

    // Warn on .env reads — values will be visible but Write requires reading first.
    // Bash commands that dump values are blocked separately in env_guard.
    let filename = file_path.rsplit('/').next().map_or(file_path, |n| n);
    let is_dotenv = filename == ".env"
        || filename.starts_with(".env.")
        || filename.to_lowercase().ends_with(".env")
        || std::path::Path::new(filename)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("env"));
    if is_dotenv {
        let context = format!(
            "[ENV_READ] file: {file_path}\n\
             status: allowed for Write purposes only\n\
             WARNING: Do not log, print, or surface any values from this file.\n\
             Use values only to determine existing keys before appending new ones.\n\
             Immediately use Write tool after reading — do not store or reference values."
        );
        drop(kavach_hook::exit_pre_tool_allow(Some(&context)));
        return;
    }

    if kavach_config::is_warn_path(file_path) {
        let context = format!("sensitive file: {file_path} — proceed with caution");
        drop(kavach_hook::exit_pre_tool_allow(Some(&context)));
        return;
    }

    drop(kavach_hook::exit_pre_tool_allow(None));
}
