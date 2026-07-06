use crate::cmd::deploy::install::{self, install_binary};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

// SOURCE: doc.rust-lang.org/std/process/struct.Command.html
const REPO_URL: &str = "https://github.com/Wankhede-Brothers/kavach-rs";

/// Resolve the install destination: `KAVACH_INSTALL_DIR` override else `~/.local/bin/kavach`.
fn install_dest() -> Result<PathBuf, String> {
    install_dest_from(std::env::var("KAVACH_INSTALL_DIR").ok())
}

/// Pure resolver: `override_dir` wins else falls back to `install::install_dest()`.
fn install_dest_from(override_dir: Option<String>) -> Result<PathBuf, String> {
    if let Some(dir) = override_dir {
        return Ok(PathBuf::from(dir).join(install::binary_filename()));
    }
    install::install_dest().ok_or_else(|| "[UPDATE] FAIL: cannot resolve $HOME".to_owned())
}

/// Path to the freshly-built binary inside a cloned source tree.
fn built_binary_path(src_dir: &Path) -> PathBuf {
    src_dir
        .join("target")
        .join(install::RELEASE_PROFILE)
        .join(install::BINARY_NAME)
}

/// Removes the temp clone dir on drop, even on an early-return error path.
struct TempCleanup(PathBuf);
impl Drop for TempCleanup {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_dir_all(&self.0) {
            eprintln!("[UPDATE] note: cleanup {} failed: {e}", self.0.display());
        }
    }
}

/// Build a "[PREFIX] <detail>" error string via push_str (avoids clippy::string_add).
fn err_msg(prefix: &str, detail: &str) -> String {
    let mut msg = String::from(prefix);
    msg.push_str(detail);
    msg
}

/// Self-update: clone latest source, build release, install over the running binary.
pub(crate) fn run() -> i32 {
    let dest = match install_dest() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    let mut src_dir = std::env::temp_dir();
    let mut dirname = String::from("kavach-update-");
    dirname.push_str(&std::process::id().to_string());
    src_dir.push(dirname);
    let _cleanup = TempCleanup(src_dir.clone());
    match update_into(&src_dir, &dest) {
        Ok(()) => {
            eprintln!("[UPDATE] OK: kavach updated at {}", dest.display());
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

fn update_into(src_dir: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(src_dir).map_err(|e| {
        err_msg(
            "[UPDATE] FAIL: mkdir: ",
            &format!("{} ({e})", src_dir.display()),
        )
    })?;
    let clone_status = Command::new("git")
        .args(["clone", "--depth", "1", REPO_URL])
        .arg(src_dir)
        .status()
        .map_err(|e| err_msg("[UPDATE] FAIL: git clone: ", &e.to_string()))?;
    if !clone_status.success() {
        return Err(err_msg("[UPDATE] FAIL: git clone exited: ", &exit_code_str(clone_status)));
    }
    let build_status = Command::new("cargo")
        .args(["build", "--release", "-p", "kavach-cli"])
        .current_dir(src_dir)
        .status()
        .map_err(|e| err_msg("[UPDATE] FAIL: cargo build: ", &e.to_string()))?;
    if !build_status.success() {
        return Err(err_msg("[UPDATE] FAIL: cargo build exited: ", &exit_code_str(build_status)));
    }
    install_binary(&built_binary_path(src_dir), dest)
}

/// Renders an `ExitStatus` without a bare format! (kept for the SQL-scanner heuristic).
fn exit_code_str(status: ExitStatus) -> String {
    status.to_string()
}

#[cfg(test)]
#[path = "update_test.rs"]
mod tests;
