//! File content wrappers (bat, difft) — read + diff.
use std::path::Path;
use std::process::Command;

use crate::toolbelt::Tool;

/// Read a file with optional line range restriction.
///
/// # Errors
/// Returns `io::Error` when bat is not installed or the command fails.
pub fn read_file<P: AsRef<Path>>(
    path: P,
    line_range: Option<(usize, usize)>,
) -> std::io::Result<std::process::Output> {
    let tool = Tool::Bat;
    if !tool.is_available() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{} not installed. Run: cargo install bat", tool.program()),
        ));
    }
    let mut cmd = Command::new(tool.program());
    cmd.arg("-p");
    if let Some((start, end)) = line_range {
        cmd.arg("-r").arg(format!("{start}:{end}"));
    }
    cmd.arg(path.as_ref()).output()
}

/// Diff two files using difftastic.
///
/// # Errors
/// Returns `io::Error` when difftastic is not installed or the command fails.
pub fn diff<P: AsRef<Path>>(old: P, new: P) -> std::io::Result<std::process::Output> {
    let tool = Tool::Difft;
    if !tool.is_available() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "{} not installed. Run: cargo install difftastic",
                tool.program()
            ),
        ));
    }
    Command::new(tool.program())
        .arg(old.as_ref())
        .arg(new.as_ref())
        .output()
}
