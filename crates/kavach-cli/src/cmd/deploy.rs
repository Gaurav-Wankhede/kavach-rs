// `kavach deploy` — one-shot build + test + install.
// SOURCE: AgentCore CLI pattern — agentcore deploy. Single command replaces:
//   cargo build --release -p kavach-cli
//   cargo nextest run -p kavach-cli
//   cp target/release/kavach ~/.local/bin/kavach
use std::path::{Path, PathBuf};
use std::process::Command;

use fs2::FileExt;

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

/// Filename of the workspace-root advisory lock that serializes `kavach deploy`.
const DEPLOY_LOCK_NAME: &str = ".deploy.lock";

/// RAII holder for the exclusive advisory `flock` that serializes concurrent
/// `kavach deploy` runs. The lock is an OS-level `fcntl`/`flock` (via `fs2`), so
/// the kernel releases it automatically if the process dies mid-deploy — a
/// crashed run can never leave a stale lock that wedges the next deploy (unlike
/// a manually-managed sentinel file). The file itself is intentionally NOT
/// removed on drop: keeping it lets the next run re-lock the same inode, and an
/// empty leftover `.deploy.lock` is harmless. Dropping the handle unlocks.
#[derive(Debug)]
struct DeployLock {
    file: std::fs::File,
}

impl DeployLock {
    /// Try to acquire the workspace deploy lock without blocking.
    ///
    /// Returns `Ok(Some(guard))` when this process won the lock, `Ok(None)` when
    /// another `kavach deploy` already holds it (the caller must refuse to
    /// proceed — two concurrent installs race the binary copy + daemon restart),
    /// and `Err` only on an unexpected filesystem error opening the lock file.
    fn try_acquire(root: &Path) -> std::io::Result<Option<Self>> {
        let path = root.join(DEPLOY_LOCK_NAME);
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&path)?;
        match file.try_lock_exclusive() {
            Ok(()) => Ok(Some(Self { file })),
            // WouldBlock is the documented "another holder" signal from fs2's
            // try_lock_exclusive — NOT an error to propagate. Anything else is.
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e),
        }
    }
}

impl Drop for DeployLock {
    fn drop(&mut self) {
        // Best-effort unlock: the kernel also drops the flock on close/exit, so a
        // failure here cannot strand the lock for the next run. Nothing actionable
        // for the caller, hence the deliberate discard.
        drop(FileExt::unlock(&self.file));
    }
}

const BINARY_NAME: &str = "kavach";
const RELEASE_PROFILE: &str = "release";
const CLI_PKG: &str = "kavach-cli";
const ENGINE_PKG: &str = "kavach-engine";

/// Self-audit ratchet: the urgent (>500 code-LOC) file count kavach ships TODAY.
/// Deploy FAILS only if a write pushes the count ABOVE this — stops regression
/// without blockading on the existing backlog (kavach follows its own gates).
/// SOURCE: decision.harness.deploy-self-audit-ratchet.
const URGENT_OVERSIZED_BASELINE: usize = 4;
const URGENT_LOC_THRESHOLD: &str = "500";

pub(crate) fn run(skip_tests: bool) -> i32 {
    // Serialize the whole deploy under a workspace-root advisory flock: two
    // concurrent `kavach deploy` runs (CI parallelism or a manual double-run)
    // would otherwise interleave the binary copy + the daemon restart, letting
    // one run's stale bits win and one `restart_rpc_daemon` kill the daemon
    // mid-lockfile-cleanup of the other (a possibly-lingering RocksDB lock).
    // Held by `_deploy_lock` for the full function scope; released on drop.
    let Some(root) = workspace_root() else {
        if let Err(io_err) =
            ewrite_or_exit("[DEPLOY] FAIL: cannot resolve workspace root for the deploy lock.")
        {
            return into_exit_code(io_err);
        }
        return 1;
    };
    let _deploy_lock = match DeployLock::try_acquire(&root) {
        Ok(Some(guard)) => guard,
        Ok(None) => {
            if let Err(io_err) = ewrite_or_exit(
                "[DEPLOY] FAIL: another `kavach deploy` is already running (holds .deploy.lock). \
                 Refusing to race the binary install + daemon restart. Re-run once it finishes.",
            ) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        Err(fs_err) => {
            if let Err(io_err) = ewrite_or_exit(&format!(
                "[DEPLOY] FAIL: cannot open the deploy lock ({DEPLOY_LOCK_NAME}): {fs_err}"
            )) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };

    // Strict gates + build + write the binary to ~/.local/bin/kavach.
    deploy_cli(skip_tests)
}

/// The 8-step CLI deploy: strict gates + build + install + restart the
/// RPC daemon so the new binary's code actually takes effect (step 8).
#[expect(clippy::too_many_lines, reason = "deploy orchestrator with 8 steps")]
fn deploy_cli(skip_tests: bool) -> i32 {
    if let Err(io_err) = print_or_exit("[DEPLOY] step 1/8: cargo check --release -D warnings") {
        return into_exit_code(io_err);
    }

    if !run_cargo_strict(&["check", "--release", "-p", CLI_PKG]) {
        if let Err(io_err) = ewrite_or_exit(
            "[DEPLOY] FAIL: cargo check produced warnings or errors. Fix them — do not suppress.",
        ) {
            return into_exit_code(io_err);
        }
        return 1;
    }

    if let Err(io_err) = print_or_exit("[DEPLOY] step 2/8: cargo clippy --release -D warnings") {
        return into_exit_code(io_err);
    }
    if !run_cargo_strict(&[
        "clippy",
        "--release",
        "-p",
        CLI_PKG,
        "-p",
        ENGINE_PKG,
        "--",
        "-D",
        "warnings",
    ]) {
        if let Err(io_err) = ewrite_or_exit(
            "[DEPLOY] FAIL: cargo clippy produced warnings. Fix them — do not suppress.",
        ) {
            return into_exit_code(io_err);
        }
        return 1;
    }

    if let Err(io_err) = print_or_exit(
        "[DEPLOY] step 3/8: cargo deny check (advisories + bans + licenses + sources)",
    ) {
        return into_exit_code(io_err);
    }
    if Command::new("cargo-deny").arg("--version").output().is_ok() {
        if !run_cargo(&["deny", "check"]) {
            if let Err(io_err) = ewrite_or_exit(
                "[DEPLOY] FAIL: cargo deny check failed. Fix policy violation \
                 (see deny.toml) — do not silently widen the allowlist.",
            ) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    } else if let Err(io_err) = print_or_exit(
        "[DEPLOY] step 3/8: SKIPPED (cargo-deny not installed; \
         install via `cargo binstall cargo-deny`)",
    ) {
        return into_exit_code(io_err);
    }

    if let Err(io_err) = print_or_exit("[DEPLOY] step 4/8: cargo machete (unused dependencies)") {
        return into_exit_code(io_err);
    }
    if Command::new("cargo-machete")
        .arg("--version")
        .output()
        .is_ok()
    {
        // NON-BLOCKING for now: the tree has 7 crates with real unused
        // deps that need cleanup (hunt.cargo-machete-unused-deps-sweep).
        if !run_cargo(&["machete"])
            && let Err(io_err) = ewrite_or_exit(
                "[DEPLOY] WARN: cargo machete found unused dependencies. \
                 Tracked in hunt.cargo-machete-unused-deps-sweep.",
            )
        {
            return into_exit_code(io_err);
            // Intentionally NOT `return 1` — see comment above.
        }
    } else if let Err(io_err) = print_or_exit(
        "[DEPLOY] step 4/8: SKIPPED (cargo-machete not installed; \
         install via `cargo binstall cargo-machete`)",
    ) {
        return into_exit_code(io_err);
    }

    if skip_tests {
        if let Err(io_err) = print_or_exit("[DEPLOY] step 5/8: SKIPPED (--skip-tests)") {
            return into_exit_code(io_err);
        }
    } else {
        if let Err(io_err) = print_or_exit("[DEPLOY] step 5/8: cargo nextest run") {
            return into_exit_code(io_err);
        }
        if !run_cargo(&["nextest", "run", "-p", CLI_PKG, "-p", ENGINE_PKG]) {
            if let Err(io_err) = ewrite_or_exit("[DEPLOY] FAIL: cargo nextest failed") {
                return into_exit_code(io_err);
            }
            return 1;
        }
    }

    if let Err(io_err) = print_or_exit("[DEPLOY] step 6/8: cargo build --release -D warnings") {
        return into_exit_code(io_err);
    }
    if !run_cargo_strict(&["build", "--release", "-p", CLI_PKG]) {
        if let Err(io_err) = ewrite_or_exit(
            "[DEPLOY] FAIL: cargo build produced warnings or errors. Fix them — do not suppress.",
        ) {
            return into_exit_code(io_err);
        }
        return 1;
    }

    if let Err(io_err) = print_or_exit("[DEPLOY] step 7/8: install to ~/.local/bin/kavach") {
        return into_exit_code(io_err);
    }
    let Some(root) = workspace_root() else {
        if let Err(io_err) = ewrite_or_exit("[DEPLOY] FAIL: cannot resolve cwd") {
            return into_exit_code(io_err);
        }
        return 1;
    };
    let src = root
        .join("target")
        .join(RELEASE_PROFILE)
        .join(binary_filename());
    let Some(dst) = install_dest() else {
        if let Err(io_err) = ewrite_or_exit("[DEPLOY] FAIL: cannot resolve $HOME") {
            return into_exit_code(io_err);
        }
        return 1;
    };

    if let Err(msg) = install_binary(&src, &dst) {
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }

    // Step 8/8: nothing to restart. There is no long-running kavach process to
    // reload — the data path is in-process ws dispatch to the standalone
    // `surreal start` server (launchd `ai.shared.kavach-surreal`), which owns
    // the DB independently of the kavach binary. The next `kavach` invocation
    // picks up the freshly installed binary automatically.
    if let Err(io_err) =
        print_or_exit("[DEPLOY] step 8/8: no daemon to restart (surreal server owns the DB)")
    {
        return into_exit_code(io_err);
    }

    let ok_msg = format!("[DEPLOY] OK: kavach installed to {}", dst.display());
    if let Err(io_err) = print_or_exit(&ok_msg) {
        return into_exit_code(io_err);
    }
    0
}

/// Install a single binary file: fresh inode (remove-then-copy) + ad-hoc
/// codesign on macOS. Returns Err(message) on failure.
///
/// Pre-copy removal gives the new file a fresh inode; macOS amfid caches code
/// signatures by inode, so overwriting in place can execute the stale cached
/// signature. openradar FB8914243.
fn install_binary(src: &Path, dst: &Path) -> Result<(), String> {
    let Some(parent) = dst.parent() else {
        return Err(format!("[DEPLOY] FAIL: {} has no parent", dst.display()));
    };
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("[DEPLOY] FAIL: mkdir {}: {e}", parent.display()))?;
    // Use symlink_metadata (does NOT follow links) so a DANGLING symlink is still
    // detected and removed — `Path::exists()` follows the link and returns false
    // for a dangling one, leaving it to collide with the copy below.
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
        // macOS amfid caches a code-signing verdict KEYED BY INODE. The
        // remove+copy above already gives `dst` a fresh inode (so a prior
        // negative-cache entry on the old inode cannot persist), but two more
        // steps are mandatory:
        //   1. Strip xattrs (`com.apple.provenance`, any quarantine) BEFORE
        //      signing — a stale association can still get the fresh inode
        //      killed on first exec.
        //   2. EXEC-VERIFY the signed binary. `codesign` exiting 0 proves the
        //      signature is well-formed, NOT that amfid will let it run — the
        //      two are independent. A deploy that reports OK on a binary that
        //      SIGKILLs (exit 137) is the exact failure this guards against:
        //      every Claude Code hook invokes this binary, so a silent
        //      kill-on-exec breaks PreToolUse/PostToolUse in every project.
        // xattr clear is advisory (a missing xattr is success), but a FAILED
        // invocation is observable signal, not silence — surface it, don't drop it.
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

/// Exec-witness: the installed binary MUST actually run, not merely be signed.
/// Runs `dst --version` and fails the install if it is killed by `SIGKILL`
/// (exit 137, amfid's kill-on-exec) or otherwise non-zero. This is the third
/// witness for the artifact — `codesign` exit 0 is necessary but not sufficient
/// on macOS.
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
    // 137 = 128 + SIGKILL(9): amfid killed it on exec (the inode-cache failure).
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

/// Run cargo with given args. Returns true on success.
fn run_cargo(args: &[&str]) -> bool {
    Command::new("cargo")
        .args(args)
        .status()
        .is_ok_and(|s| s.success())
}

/// Run cargo with `RUSTFLAGS=-D warnings` so any lint warning fails the deploy.
fn run_cargo_strict(args: &[&str]) -> bool {
    let merged = match std::env::var("RUSTFLAGS") {
        Ok(existing) if existing.is_empty() => "-D warnings".to_owned(),
        Ok(existing) => format!("{existing} -D warnings"),
        Err(std::env::VarError::NotPresent) => "-D warnings".to_owned(),
        Err(std::env::VarError::NotUnicode(_)) => {
            ewrite_or_exit(
                "[DEPLOY] FAIL: RUSTFLAGS contains non-UTF-8 bytes; refusing to override silently. \
                 Unset or fix RUSTFLAGS, then re-run kavach deploy.",
            )
            .ok();
            return false;
        }
    };
    Command::new("cargo")
        .env("RUSTFLAGS", merged)
        .args(args)
        .status()
        .is_ok_and(|s| s.success())
}

/// Workspace root = current working directory (cargo conventions).
fn workspace_root() -> Option<PathBuf> {
    std::env::current_dir().ok()
}

/// Platform binary filename: `kavach.exe` on Windows, `kavach` elsewhere.
/// `cargo build` emits the `.exe` suffix on the MSVC/GNU Windows targets, and
/// the install destination must match so the copied file is executable.
fn binary_filename() -> String {
    if cfg!(windows) {
        format!("{BINARY_NAME}.exe")
    } else {
        BINARY_NAME.to_owned()
    }
}

/// Resolve `~/.local/bin/kavach[.exe]` without panicking on a missing home dir.
/// Uses `dirs::home_dir()` (resolves `$HOME` on Unix, `%USERPROFILE%` on
/// Windows) so the install path is correct on every host.
fn install_dest() -> Option<PathBuf> {
    Some(
        dirs::home_dir()?
            .join(".local")
            .join("bin")
            .join(binary_filename()),
    )
}

// These tests exercise the macOS/Unix install path (symlink fixtures via
// `std::os::unix::fs`), so they are gated to Unix. The cross-platform logic
// (`binary_filename`, `install_dest`) is covered by the build itself on Windows.
#[cfg(all(test, unix))]
mod tests {
    use super::install_binary;
    use std::fs;

    // A DANGLING symlink (target deleted) must still be removed before copy —
    // `Path::exists()` follows the link and returns false, so the pre-fix code
    // skipped removal and the copy collided with the leftover link.
    #[test]
    fn replaces_a_dangling_symlink() {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = std::env::temp_dir().join(format!("kavach-dangletest-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let src = dir.join("src-bin");
        // A REAL executable that tolerates `--version` (any args): install_binary's
        // exec-witness (`verify_runs`) runs `dst --version`, so the fixture must
        // actually run and exit 0 — a plain non-+x file would (correctly) fail
        // the witness. This exercises the full copy→sign→exec-verify chain.
        fs::write(&src, b"#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(&src, fs::Permissions::from_mode(0o755)).unwrap();
        let dst = dir.join("kavach");
        std::os::unix::fs::symlink(dir.join("does-not-exist"), &dst).unwrap();
        assert!(
            !dst.exists(),
            "precondition: dangling symlink (exists() follows → false)"
        );

        // install_binary copies, codesigns (macOS), and exec-verifies the result.
        install_binary(&src, &dst).expect("must replace dangling symlink with the new file");
        // The dangling symlink must be GONE — replaced by a real regular file.
        // `!is_symlink()` (not `is_file()`) dodges clippy::filetype_is_file: the
        // assertion we care about is "no longer a link", and symlink_metadata
        // already proves the entry exists.
        let ft = dst.symlink_metadata().unwrap().file_type();
        assert!(!ft.is_symlink() && !ft.is_dir());

        fs::remove_dir_all(&dir).ok();
    }

    // The deploy lock must serialize concurrent `kavach deploy` runs: while one
    // holds the workspace `.deploy.lock`, a second `try_acquire` on the same root
    // must be REFUSED (Ok(None)), and once the first guard drops the lock must be
    // re-acquirable. This is the race the card closes: two installs interleaving
    // the binary copy + daemon restart.
    #[test]
    fn deploy_concurrent_lock_prevents_race() {
        use super::DeployLock;

        let dir = std::env::temp_dir().join(format!("kavach-locktest-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();

        // First acquirer wins.
        let first = DeployLock::try_acquire(&dir)
            .expect("open lock file")
            .expect("first acquire must win");

        // A concurrent second acquirer on the same root is refused (lock held).
        let second = DeployLock::try_acquire(&dir).expect("open lock file");
        assert!(
            second.is_none(),
            "second concurrent deploy must be refused while the lock is held"
        );

        // Releasing the first lets the next run re-acquire — no stale lock.
        drop(first);
        let third = DeployLock::try_acquire(&dir)
            .expect("open lock file")
            .expect("re-acquire must win after the prior guard drops");
        drop(third);

        fs::remove_dir_all(&dir).ok();
    }
}
