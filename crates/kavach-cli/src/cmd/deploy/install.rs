use std::path::{Path, PathBuf};
use std::process::Command;

pub(super) const BINARY_NAME: &str = "kavach";
pub(super) const RELEASE_PROFILE: &str = "release";

/// Install a single binary file: fresh inode (remove-then-copy) + codesign on macOS.
pub(super) fn install_binary(src: &Path, dst: &Path) -> Result<(), String> {
    let Some(parent) = dst.parent() else {
        return Err(format!("[DEPLOY] FAIL: {} has no parent", dst.display()));
    };
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("[DEPLOY] FAIL: mkdir {}: {e}", parent.display()))?;
    if dst.symlink_metadata().is_ok() {
        std::fs::remove_file(dst)
            .map_err(|e| format!("[DEPLOY] FAIL: unlink {}: {e}", dst.display()))?;
    }
    std::fs::copy(src, dst).map_err(|e| {
        format!(
            "[DEPLOY] FAIL: copy {} -> {}: {e}",
            src.display(),
            dst.display()
        )
    })?;
    if cfg!(target_os = "macos") {
        match Command::new("xattr").args(["-c"]).arg(dst).status() {
            Ok(s) if s.success() => {}
            Ok(s) => eprintln!("[DEPLOY] note: xattr -c {} exited {s}", dst.display()),
            Err(e) => eprintln!("[DEPLOY] note: xattr -c unavailable ({e}); continuing"),
        }
        let signed = Command::new("codesign")
            .args(["--force", "--sign", "-"])
            .arg(dst)
            .status()
            .is_ok_and(|s| s.success());
        if !signed {
            return Err(format!(
                "[DEPLOY] FAIL: codesign {}; binary will be killed by amfid on exec. \
                 Run `codesign --force --sign - {}` manually.",
                dst.display(),
                dst.display()
            ));
        }
        verify_runs(dst)?;
    }
    Ok(())
}

/// Exec-witness: installed binary MUST run, not merely be signed; runs dst --version.
fn verify_runs(dst: &Path) -> Result<(), String> {
    let out = Command::new(dst).arg("--version").output().map_err(|e| {
        format!(
            "[DEPLOY] FAIL: installed binary {} could not be executed: {e}",
            dst.display()
        )
    })?;
    if out.status.success() {
        return Ok(());
    }
    let code = out.status.code().unwrap_or(-1);
    Err(format!(
        "[DEPLOY] FAIL: installed binary {} exits {code} on `--version` \
         (137 = amfid SIGKILL on exec). The signature is on disk but amfid \
         refused it; every Claude Code hook would be killed. Re-run the deploy \
         (a fresh inode + re-sign usually clears the amfid cache), or run \
         `rm {} && cp <target/release/kavach> {} && xattr -c {} && \
         codesign --force --sign - {}` manually.",
        dst.display(),
        dst.display(),
        dst.display(),
        dst.display(),
        dst.display()
    ))
}

/// Platform binary filename: `kavach.exe` on Windows, `kavach` elsewhere.
pub(super) fn binary_filename() -> String {
    if cfg!(windows) {
        format!("{BINARY_NAME}.exe")
    } else {
        BINARY_NAME.to_owned()
    }
}

/// Resolve `~/.local/bin/kavach[.exe]` without panicking on a missing home dir.
pub(super) fn install_dest() -> Option<PathBuf> {
    Some(
        dirs::home_dir()?
            .join(".local")
            .join("bin")
            .join(binary_filename()),
    )
}

pub(super) use self::install_binary;
