//! Network/data wrappers (jaq, xh) — JSON query, HTTP GET, URL reachability.
use std::process::Command;

use super::tool::Tool;

/// Query JSON input using jaq.
///
/// # Errors
/// Returns `io::Error` when the command fails to spawn or complete.
pub fn json_query(query: &str, input: &str) -> std::io::Result<std::process::Output> {
    let tool = Tool::Jaq;
    let program = tool.resolve();
    Command::new(program)
        .arg(query)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                stdin.write_all(input.as_bytes())?;
            }
            child.wait_with_output()
        })
}

/// Perform an HTTP GET request using xh.
///
/// # Errors
/// Returns `io::Error` when xh is not installed or the command fails.
pub fn http_get(url: &str) -> std::io::Result<std::process::Output> {
    let tool = Tool::Xh;
    if !tool.is_available() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{} not installed. Run: cargo install xh", tool.program()),
        ));
    }
    Command::new(tool.program()).arg("GET").arg(url).output()
}

/// Check URL reachability. Returns Ok(true) for 2xx/3xx, Ok(false) for 4xx/5xx.
///
/// Uses xh (Rust). No fallback — toolbelt required.
/// SOURCE: <https://crates.io/crates/xh> — supports --timeout <SEC> and --check-status.
/// xh exit code reflects HTTP status when --check-status is set:
///   0 = 2xx/3xx, 4 = 4xx, 5 = 5xx, other = network error.
/// This avoids parsing headers (HTTP/1.1 vs HTTP/2 :status: format differs).
///
/// # Errors
/// Returns `Err` when xh is not installed, the command fails, or an unexpected exit code is returned.
pub fn verify_url_reachable(url: &str, timeout_secs: u32) -> Result<bool, String> {
    let tool = Tool::Xh;
    if !tool.is_available() {
        return Err(format!(
            "{} not installed. Run: cargo install xh",
            tool.program()
        ));
    }
    let status = Command::new(tool.program())
        .args([
            "HEAD",
            "--check-status",
            "--timeout",
            &timeout_secs.to_string(),
            "--ignore-stdin",
            url,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    match status.code() {
        Some(0) => Ok(true),
        Some(4 | 5) => Ok(false),
        Some(c) => Err(format!("xh exited with code {c}")),
        None => Err("xh terminated by signal".into()),
    }
}
