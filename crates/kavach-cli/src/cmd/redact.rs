//! Redact environment files key-preserving or mask secrets from file/stdin.
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code};
use std::fs;
use std::io::{self, Read, Write};

/// Run the `kavach redact [FILE]` command.
pub(crate) fn run(file_opt: Option<&str>) -> i32 {
    let (content, file_hint) = match file_opt {
        Some("-") | None => {
            let mut buf = String::new();
            if let Err(e) = io::stdin().read_to_string(&mut buf) {
                let msg = format!("[kavach redact] ERROR reading stdin: {e}");
                return match ewrite_or_exit(&msg) {
                    Ok(()) => 1,
                    Err(io_err) => into_exit_code(io_err),
                };
            }
            (buf, None)
        }
        Some(path) => match fs::read_to_string(path) {
            Ok(c) => (c, Some(path)),
            Err(e) => {
                let msg = format!("[kavach redact] ERROR reading {path}: {e}");
                return match ewrite_or_exit(&msg) {
                    Ok(()) => 1,
                    Err(io_err) => into_exit_code(io_err),
                };
            }
        },
    };

    let sanitized = kavach_engine::gates::secret_redactor::sanitize_output(&content, file_hint);
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    if let Err(e) = handle.write_all(sanitized.as_bytes()) {
        return into_exit_code(e.into());
    }
    if !sanitized.ends_with('\n')
        && let Err(e) = handle.write_all(b"\n")
    {
        return into_exit_code(e.into());
    }
    0
}
