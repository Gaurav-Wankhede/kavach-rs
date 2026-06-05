//! Directory inspection wrappers (erd/tree, tokei, dust).
use std::path::Path;
use std::process::Command;

use crate::toolbelt::Tool;

/// Display a directory tree to a specified depth using erd or tree.
///
/// # Errors
/// Returns `io::Error` when the command fails.
pub fn tree<P: AsRef<Path>>(dir: P, depth: usize) -> std::io::Result<std::process::Output> {
    let tool = Tool::Erd;
    if tool.is_available() {
        Command::new(tool.program())
            .arg("-L")
            .arg(depth.to_string())
            .arg(dir.as_ref())
            .output()
    } else {
        Command::new("tree")
            .arg("-L")
            .arg(depth.to_string())
            .arg(dir.as_ref())
            .output()
    }
}

/// Count lines of code in a directory using tokei.
///
/// # Errors
/// Returns `io::Error` when tokei is not installed or the command fails.
pub fn count_lines<P: AsRef<Path>>(dir: P) -> std::io::Result<std::process::Output> {
    let tool = Tool::Tokei;
    if !tool.is_available() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{} not installed. Run: cargo install tokei", tool.program()),
        ));
    }
    Command::new(tool.program())
        .arg("-o")
        .arg("json")
        .arg(dir.as_ref())
        .output()
}

/// Display disk usage in a directory using dust.
///
/// # Errors
/// Returns `io::Error` when dust is not installed or the command fails.
pub fn disk_usage<P: AsRef<Path>>(dir: P, depth: usize) -> std::io::Result<std::process::Output> {
    let tool = Tool::Dust;
    if !tool.is_available() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "{} not installed. Run: cargo install du-dust",
                tool.program()
            ),
        ));
    }
    Command::new(tool.program())
        .arg("-d")
        .arg(depth.to_string())
        .arg(dir.as_ref())
        .output()
}
