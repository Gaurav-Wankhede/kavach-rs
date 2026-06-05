use crate::error::internal;
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockfileContent {
    pub port: u16,
    pub pid: u32,
    pub started_at: String,
    pub transport: String,
}

#[must_use]
pub fn default_lockfile_path() -> PathBuf {
    if cfg!(target_os = "macos") {
        dirs::home_dir().map_or_else(
            || PathBuf::from("/tmp/kavach-rpc.port"),
            |h| h.join("Library/Application Support/SharedAI/kavach-rpc.port"),
        )
    } else if cfg!(target_os = "windows") {
        dirs::data_local_dir().map_or_else(
            || PathBuf::from("C:\\Users\\Public\\SharedAI\\kavach-rpc.port"),
            |d| d.join("SharedAI\\kavach-rpc.port"),
        )
    } else {
        dirs::data_dir().map_or_else(
            || PathBuf::from("/tmp/kavach-rpc.port"),
            |d| d.join("shared-ai/kavach-rpc.port"),
        )
    }
}

/// Write a lockfile containing the RPC port and PID.
///
/// # Errors
///
/// Returns an error if the lockfile directory cannot be created, the file cannot be written,
/// or if another kavach-rpc process is already running on the given port.
pub fn write_lockfile(port: u16, transport: &str) -> Result<PathBuf, ErrorObjectOwned> {
    let path = default_lockfile_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| internal(format!("create lockfile dir: {e}")))?;
    }

    if path.exists()
        && let Ok(stale) = read_lockfile()
    {
        if is_pid_alive(stale.pid) {
            return Err(internal(format!(
                "kavach-rpc already running (pid {}, port {})",
                stale.pid, stale.port
            )));
        } else if let Err(e) = std::fs::remove_file(&path) {
            // Stale-lockfile cleanup failure: log + proceed. The
            // subsequent fs::write will surface the real conflict
            // (e.g. EACCES) with full Result propagation.
            tracing::warn!(target: "kavach_rpc::lockfile", path = %path.display(), error = %e, "stale cleanup failed");
        }
    }

    let content = LockfileContent {
        port,
        pid: std::process::id(),
        started_at: chrono::Utc::now().to_rfc3339(),
        transport: transport.to_owned(),
    };

    let json = serde_json::to_string_pretty(&content)
        .map_err(|e| internal(format!("serialize lockfile: {e}")))?;
    std::fs::write(&path, json).map_err(|e| internal(format!("write lockfile: {e}")))?;
    Ok(path)
}

pub fn remove_lockfile() {
    let path = default_lockfile_path();
    if let Err(e) = std::fs::remove_file(&path) {
        // Shutdown-time cleanup; if removal fails (NotFound is benign,
        // EACCES means manual intervention needed) log it and return.
        // We deliberately do not propagate — shutdown must complete.
        if e.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(target: "kavach_rpc::lockfile", path = %path.display(), error = %e, "remove_lockfile failed");
        }
    }
}

/// Read the lockfile containing RPC port and PID information.
///
/// # Errors
///
/// Returns an error if the lockfile cannot be read or parsed.
pub fn read_lockfile() -> Result<LockfileContent, ErrorObjectOwned> {
    let path = default_lockfile_path();
    let bytes = std::fs::read(&path).map_err(|e| internal(format!("read lockfile: {e}")))?;
    serde_json::from_slice(&bytes).map_err(|e| internal(format!("parse lockfile: {e}")))
}

#[cfg(unix)]
fn is_pid_alive(pid: u32) -> bool {
    use std::process::Command;
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .is_ok_and(|s| s.success())
}

// Windows lacks `kill -0`; PID-liveness check is platform-specific. Until a
// proper OpenProcess / GetExitCodeProcess path is wired (see kavach-rpc roadmap),
// log the PID we couldn't probe and assume the process is alive so we never
// falsely steal the lock. The pid is consumed by tracing for diagnosability.
#[cfg(windows)]
fn is_pid_alive(pid: u32) -> bool {
    tracing::debug!(target: "kavach_rpc::lockfile", pid, "is_pid_alive: windows stub — conservative true");
    true
}
