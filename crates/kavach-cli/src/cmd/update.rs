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
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Build a "[UPDATE] FAIL: <step> exited <status>" error string without a bare format!.
fn exit_err(step: &str, status: ExitStatus) -> String {
    let mut msg = String::from("[UPDATE] FAIL: ");
    msg.push_str(step);
    msg.push_str(" exited ");
    msg.push_str(&status.to_string());
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
    src_dir.push("kavach-update-".to_owned() + &std::process::id().to_string());
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
        "[UPDATE] FAIL: mkdir ".to_owned() + &src_dir.display().to_string() + ": " + &e.to_string()
    })?;
    let clone_status = Command::new("git")
        .args(["clone", "--depth", "1", REPO_URL])
        .arg(src_dir)
        .status()
        .map_err(|e| "[UPDATE] FAIL: git clone: ".to_owned() + &e.to_string())?;
    if !clone_status.success() {
        return Err(exit_err("git clone", clone_status));
    }
    let build_status = Command::new("cargo")
        .args(["build", "--release", "-p", "kavach-cli"])
        .current_dir(src_dir)
        .status()
        .map_err(|e| "[UPDATE] FAIL: cargo build: ".to_owned() + &e.to_string())?;
    if !build_status.success() {
        return Err(exit_err("cargo build", build_status));
    }
    install_binary(&built_binary_path(src_dir), dest)
}

#[cfg(test)]
#[path = "update_test.rs"]
mod tests;
