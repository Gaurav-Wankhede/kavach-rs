//! Pattern/file/AST search wrappers (rg, fd, sg).
use std::path::Path;
use std::process::Command;

use super::tool::Tool;

/// Search for a pattern in a directory.
///
/// # Errors
/// Returns `io::Error` when ripgrep is not installed or the command fails.
pub fn search<P: AsRef<Path>>(pattern: &str, dir: P) -> std::io::Result<std::process::Output> {
    let tool = Tool::Rg;
    if !tool.is_available() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "{} not installed. Run: cargo install ripgrep",
                tool.program()
            ),
        ));
    }
    Command::new(tool.program())
        .args(["-n", pattern])
        .arg(dir.as_ref())
        .output()
}

