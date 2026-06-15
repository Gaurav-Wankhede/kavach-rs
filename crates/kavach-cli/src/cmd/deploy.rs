// `kavach deploy` — one-shot build + test + install.
// SOURCE: AgentCore CLI pattern — agentcore deploy. Single command replaces:
//   cargo build --release -p kavach-cli
//   cargo nextest run -p kavach-cli
//   cp target/release/kavach ~/.local/bin/kavach
//
// With `--bundle` it additionally builds the KavachApp.app GUI via `dx bundle`,
// embedding the kavach CLI as a sidecar (Dioxus `external_bin`), codesigns the
// whole .app, installs it to /Applications, and symlinks ~/.local/bin/kavach
// into the bundle so the terminal CLI and GUI share one binary.
// SOURCE: https://dioxuslabs.com/learn/0.7/tutorial/bundle/
// SOURCE: https://dioxuslabs.com/learn/0.7/guides/tools/configure/
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

const BINARY_NAME: &str = "kavach";
const RELEASE_PROFILE: &str = "release";
const CLI_PKG: &str = "kavach-cli";
const ENGINE_PKG: &str = "kavach-engine";
const APP_PKG: &str = "kavach-app";
// dx derives the bundle name from the crate name (kavach-app -> KavachApp),
// not from Dioxus.toml [application] name. Verified empirically against
// `dx bundle` 0.7.7 output. SOURCE: dx bundle run, 2026-05.
const APP_BUNDLE_NAME: &str = "KavachApp.app";

pub(crate) fn run(skip_tests: bool, bundle: bool) -> i32 {
    // Track A: strict gates + build + (for a CLI-only install) write the binary
    // to ~/.local/bin/kavach. In --bundle mode the standalone install is SKIPPED:
    // the bundle track installs the CLI INTO KavachApp.app and re-points the
    // symlink, so a plain install_binary would (correctly) refuse to clobber it.
    let cli = deploy_cli(skip_tests, bundle);
    if cli != 0 {
        return cli;
    }
    // Track B (opt-in): build the GUI .app/.dmg with the CLI embedded.
    if bundle {
        return deploy_bundle();
    }
    0
}

/// Track A — the 8-step CLI deploy: strict gates + build + install + restart the
/// RPC daemon so the new binary's code actually takes effect (step 8).
#[expect(clippy::too_many_lines, reason = "deploy orchestrator with 8 steps")]
fn deploy_cli(skip_tests: bool, bundle: bool) -> i32 {
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

    // Steps 7/8 install the standalone CLI to ~/.local/bin and restart the
    // daemon. In --bundle mode BOTH are the bundle track's job: it installs the
    // CLI into KavachApp.app, re-points the symlink, and restarts the daemon off
    // the app binary. Running install_binary here would (correctly) refuse to
    // clobber the bundle symlink, aborting the deploy before the bundle runs —
    // so skip 7/8 entirely when bundling.
    if bundle {
        if let Err(io_err) = print_or_exit(
            "[DEPLOY] step 7/8: SKIPPED (--bundle installs the CLI into KavachApp.app)",
        ) {
            return into_exit_code(io_err);
        }
        return 0;
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

    // Step 8/8: restart the long-running RPC daemon so it loads the NEW binary.
    // The install replaces the on-disk file, but a daemon started from the OLD
    // binary keeps the OLD code in memory — every gate (dispatch, stop, kanban)
    // routes through that stale process, so deploys silently never took effect.
    // ROOT CAUSE of the "stop-gate fixes don't apply" loop. Kill it; the next
    // hook respawns it on the fresh binary.
    if let Err(io_err) = print_or_exit("[DEPLOY] step 8/8: restart RPC daemon (load new binary)") {
        return into_exit_code(io_err);
    }
    restart_rpc_daemon();

    let ok_msg = format!("[DEPLOY] OK: kavach installed to {}", dst.display());
    if let Err(io_err) = print_or_exit(&ok_msg) {
        return into_exit_code(io_err);
    }
    0
}

/// Terminate the running `kavach rpc` daemon so the next hook respawns it on the
/// freshly installed binary. Best-effort: a missing/unreadable lockfile means no
/// daemon is running (nothing to restart) — never fail the deploy over it. The
/// lockfile is removed so a stale entry can't block the respawn's `write_lockfile`.
fn restart_rpc_daemon() {
    // The lockfile read + SIGTERM is Unix-only: the sync UDS daemon never runs on
    // Windows (the RPC client is `cfg(not(unix))` and the gates open SurrealDB
    // directly), so there is no pid to signal — `lock` would be an unused binding
    // there (`-D unused-variables`). On Windows we fall straight through to the
    // unconditional lockfile cleanup below, which is a harmless no-op when absent.
    #[cfg(unix)]
    {
        let Ok(lock) = kavach_rpc::lockfile::read_lockfile() else {
            // No lockfile → no daemon running. The next hook starts one fresh.
            print_or_exit("[DEPLOY] step 8/8: no running daemon (nothing to restart)").ok();
            return;
        };
        // SIGTERM lets the daemon run its shutdown (remove_lockfile, close socket).
        let killed = Command::new("kill")
            .arg(lock.pid.to_string())
            .status()
            .is_ok_and(|s| s.success());
        let msg = if killed {
            // GRACEFUL HANDOFF (race-free restart): SIGTERM is async and the
            // daemon fsyncs RocksDB on shutdown, so it may still hold the OS
            // `fcntl` LOCK for tens of ms after `kill` returns. Returning here
            // (and removing the lockfile) immediately lets the next hook spawn a
            // new daemon that opens the DB while the old one still holds the
            // LOCK -> "Resource temporarily unavailable" (the post-deploy race,
            // unit.daemon-restart-race-free). Block until the old PID is gone
            // before releasing — bounded so a wedged daemon cannot hang deploy.
            if wait_for_pid_exit(lock.pid) {
                format!(
                    "[DEPLOY] step 8/8: daemon pid {} exited (LOCK released)",
                    lock.pid
                )
            } else {
                format!(
                    "[DEPLOY] step 8/8: WARNING daemon pid {} did not exit within budget; \
                     respawn may hit transient LOCK contention (self-heals via backoff)",
                    lock.pid
                )
            }
        } else {
            format!(
                "[DEPLOY] step 8/8: daemon pid {} not running (stale lockfile)",
                lock.pid
            )
        };
        print_or_exit(&msg).ok();
    }
    // Remove the lockfile unconditionally: if the daemon was already dead the
    // entry is stale; if we just killed it (and waited for exit above), removing
    // now lets the respawn claim a clean lock without racing the dying process.
    kavach_rpc::lockfile::remove_lockfile();
}

/// Poll until process `pid` has exited (the `RocksDB` LOCK is released only when
/// the holding process is fully gone), bounded so a wedged daemon cannot hang
/// the deploy. Returns true if it exited within the budget, false on timeout.
///
/// Budget mirrors the proven daemon-eviction wait in
/// `kavach_surreal::connection::try_stop_daemon` (20 x 50ms = 1s) — long enough
/// for a SIGTERM-handled `RocksDB` fsync-and-exit, short enough that a stuck
/// daemon degrades to the self-healing backoff path rather than blocking deploy.
///
/// Unix-only: the daemon is signalled via POSIX kill; on non-unix there is no
/// daemon to wait on (the restart block is itself `#[cfg(unix)]`). Uses the
/// `kill -0 <pid>` existence probe — the same `Command::new("kill")` tool the
/// restart already shells out to, so no new crate dependency is pulled in.
#[cfg(unix)]
fn wait_for_pid_exit(pid: u32) -> bool {
    // `kill -0 <pid>` sends no signal; it only checks the process exists.
    // Exit 0 = alive, non-zero (ESRCH) = gone. One final probe after the loop
    // so a daemon exiting in the last window is not a false timeout.
    let alive = || {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .is_ok_and(|s| s.success())
    };
    for _ in 0..20 {
        if !alive() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    !alive()
}

/// Track B — build the GUI app bundle with the CLI embedded as a sidecar,
/// codesign, install to /Applications, and symlink the terminal CLI into it.
///
/// Steps:
///   B1. Resolve the target triple (sidecar filename suffix).
///   B2. Stage the freshly built CLI as crates/kavach-app/bin/kavach-<triple>.
///   B3. `dx bundle --release --platform desktop --package-types macos --package-types dmg`.
///   B4. codesign --deep --force --sign - KavachApp.app  (macOS amfid).
///   B5. Install KavachApp.app to /Applications (fresh, remove-then-copy).
///   B6. Symlink ~/.local/bin/kavach -> /Applications/KavachApp.app/Contents/MacOS/kavach.
#[expect(clippy::too_many_lines, reason = "bundle orchestrator with 6 steps")]
fn deploy_bundle() -> i32 {
    if !cfg!(target_os = "macos") {
        if let Err(io_err) = ewrite_or_exit(
            "[BUNDLE] FAIL: --bundle currently supports macOS only (codesign + .app/.dmg).",
        ) {
            return into_exit_code(io_err);
        }
        return 1;
    }
    if Command::new("dx").arg("--version").output().is_err() {
        if let Err(io_err) = ewrite_or_exit(
            "[BUNDLE] FAIL: `dx` (Dioxus 0.7 CLI) not found. \
             Install via `cargo binstall dioxus-cli` and re-run.",
        ) {
            return into_exit_code(io_err);
        }
        return 1;
    }
    let Some(root) = workspace_root() else {
        if let Err(io_err) = ewrite_or_exit("[BUNDLE] FAIL: cannot resolve cwd") {
            return into_exit_code(io_err);
        }
        return 1;
    };

    // B1: target triple.
    let triple = match host_triple() {
        Ok(t) => t,
        Err(msg) => {
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };

    // B2: stage the CLI as the triple-suffixed sidecar. The CLI was already
    // built in release by Track A (step 6/7).
    if let Err(io_err) = print_or_exit(&format!(
        "[BUNDLE] step 1/6: stage CLI sidecar (bin/kavach-{triple})"
    )) {
        return into_exit_code(io_err);
    }
    let cli_release = root.join("target").join(RELEASE_PROFILE).join(BINARY_NAME);
    let sidecar_dir = root.join("crates").join(APP_PKG).join("bin");
    let sidecar = sidecar_dir.join(format!("{BINARY_NAME}-{triple}"));
    if let Err(e) = std::fs::create_dir_all(&sidecar_dir) {
        if let Err(io_err) = ewrite_or_exit(&format!("[BUNDLE] FAIL: mkdir bin/: {e}")) {
            return into_exit_code(io_err);
        }
        return 1;
    }
    if let Err(msg) = install_binary(&cli_release, &sidecar) {
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }

    // B3: dx bundle. Run from the app crate dir so dx picks up its Dioxus.toml.
    if let Err(io_err) =
        print_or_exit("[BUNDLE] step 2/6: dx bundle --release --platform desktop (macos + dmg)")
    {
        return into_exit_code(io_err);
    }
    let app_dir = root.join("crates").join(APP_PKG);
    let dx_ok = Command::new("dx")
        .current_dir(&app_dir)
        .args([
            "bundle",
            "--release",
            "--platform",
            "desktop",
            "--package-types",
            "macos",
            "--package-types",
            "dmg",
        ])
        .status()
        .is_ok_and(|s| s.success());
    if !dx_ok {
        // The staged sidecar is a consumed build artifact; don't leave it in
        // the tree to be silently reused by a later run. Non-fatal but logged.
        cleanup_stage(&sidecar_dir);
        if let Err(io_err) = ewrite_or_exit("[BUNDLE] FAIL: dx bundle failed. See output above.") {
            return into_exit_code(io_err);
        }
        return 1;
    }
    // Bundle succeeded — dx has embedded the sidecar, so the staged copy is no
    // longer needed. Cleanup keeps the working tree clean.
    cleanup_stage(&sidecar_dir);

    // Resolve produced .app. dx 0.7.7 emits to
    // target/dx/<pkg>/bundle/macos/macos/<Name>.app (single macos/macos —
    // not the doubly-nested bundle/macos/bundle/macos the older docs show).
    // Verified empirically. SOURCE: dx bundle run, 2026-05.
    let app_src = root
        .join("target")
        .join("dx")
        .join(APP_PKG)
        .join("bundle")
        .join("macos")
        .join("macos")
        .join(APP_BUNDLE_NAME);
    if !app_src.exists() {
        if let Err(io_err) = ewrite_or_exit(&format!(
            "[BUNDLE] FAIL: expected {} after dx bundle — not found.",
            app_src.display()
        )) {
            return into_exit_code(io_err);
        }
        return 1;
    }

    // B4: codesign the whole bundle (ad-hoc). --deep signs nested binaries
    // (the embedded kavach sidecar) before the outer bundle. macOS amfid.
    if let Err(io_err) =
        print_or_exit("[BUNDLE] step 3/6: codesign --deep --force --sign - KavachApp.app")
    {
        return into_exit_code(io_err);
    }
    if !codesign_deep(&app_src) {
        if let Err(io_err) = ewrite_or_exit(&format!(
            "[BUNDLE] FAIL: codesign failed for {}. The app will be killed by \
             amfid on launch. Resign manually: codesign --deep --force --sign - {}",
            app_src.display(),
            app_src.display()
        )) {
            return into_exit_code(io_err);
        }
        return 1;
    }

    // B5: install KavachApp.app to /Applications (fresh: remove-then-copy).
    if let Err(io_err) = print_or_exit("[BUNDLE] step 4/6: install KavachApp.app to /Applications")
    {
        return into_exit_code(io_err);
    }
    let app_dst = PathBuf::from("/Applications").join(APP_BUNDLE_NAME);
    if app_dst.exists()
        && let Err(e) = std::fs::remove_dir_all(&app_dst)
    {
        if let Err(io_err) = ewrite_or_exit(&format!(
            "[BUNDLE] FAIL: remove existing {}: {e}",
            app_dst.display()
        )) {
            return into_exit_code(io_err);
        }
        return 1;
    }
    // .app is a directory tree — use `cp -R` to preserve the bundle layout +
    // symlinks + the codesignature. std::fs has no recursive copy.
    let cp_ok = Command::new("cp")
        .arg("-R")
        .arg(&app_src)
        .arg(&app_dst)
        .status()
        .is_ok_and(|s| s.success());
    if !cp_ok {
        // The destination was removed before the copy, so a mid-copy failure
        // leaves a half-populated .app at app_dst. Remove it so the next run's
        // existence check doesn't see — and trust — a corrupt bundle. Fail
        // closed: report cp's failure even if cleanup also fails.
        if let Err(e) = std::fs::remove_dir_all(&app_dst)
            && app_dst.exists()
        {
            if let Err(io_err) = ewrite_or_exit(&format!(
                "[BUNDLE] FAIL: cp -R failed AND could not clean partial {}: {e} — \
                 remove it manually before re-running.",
                app_dst.display()
            )) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        if let Err(io_err) = ewrite_or_exit(&format!(
            "[BUNDLE] FAIL: cp -R {} -> {} (partial copy cleaned up)",
            app_src.display(),
            app_dst.display()
        )) {
            return into_exit_code(io_err);
        }
        return 1;
    }

    // B6: symlink ~/.local/bin/kavach into the installed bundle so the
    // terminal CLI and the GUI's embedded sidecar are one binary.
    if let Err(io_err) = print_or_exit(
        "[BUNDLE] step 5/7: symlink ~/.local/bin/kavach -> KavachApp.app/Contents/MacOS/kavach",
    ) {
        return into_exit_code(io_err);
    }
    let embedded_cli = app_dst.join("Contents").join("MacOS").join(BINARY_NAME);
    let Some(link) = install_dest() else {
        if let Err(io_err) = ewrite_or_exit("[BUNDLE] FAIL: cannot resolve $HOME") {
            return into_exit_code(io_err);
        }
        return 1;
    };
    if let Err(msg) = symlink_force(&embedded_cli, &link) {
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }

    // B7: restart the RPC daemon so it loads the NEW embedded binary. Without
    // this, a --bundle deploy leaves the long-running daemon on the OLD code —
    // every gate routes through that stale process and the deploy silently never
    // takes effect (the same failure the CLI track's step 8 fixes).
    if let Err(io_err) =
        print_or_exit("[BUNDLE] step 6/7: restart RPC daemon (load new binary)")
    {
        return into_exit_code(io_err);
    }
    restart_rpc_daemon();

    if let Err(io_err) = print_or_exit(&format!(
        "[BUNDLE] step 7/7: OK — installed {} and symlinked CLI to {}",
        app_dst.display(),
        link.display()
    )) {
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
    // Refuse to silently clobber a `--bundle` install: there, `dst` is a symlink
    // into /Applications/KavachApp.app so the CLI and the GUI's embedded sidecar
    // are ONE signed binary. A plain `deploy` that overwrote it with a standalone
    // file would diverge the two — the CLI runs the new build, the daemon/GUI the
    // stale embedded one. Direct the user to the matching mode instead.
    if let Ok(target) = std::fs::read_link(dst)
        && target
            .components()
            .any(|c| c.as_os_str() == "KavachApp.app")
    {
        return Err(format!(
            "[DEPLOY] FAIL: {} is a symlink into KavachApp.app (a --bundle install). \
             A plain `deploy` would replace it with a standalone copy and diverge the \
             CLI from the GUI/daemon binary. Re-run `kavach deploy --bundle` to update \
             both together, or remove the symlink first to switch to a CLI-only install.",
            dst.display()
        ));
    }
    // Use symlink_metadata (does NOT follow links) so a DANGLING symlink is still
    // detected and removed — `Path::exists()` follows the link and returns false
    // for a dangling one, leaving it to collide with the copy below. Mirrors the
    // correct check already used by symlink_force.
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

/// Remove the staged-sidecar directory. Cleanup is non-fatal — a leftover
/// build artifact does not invalidate the bundle — but the failure is logged
/// so a persistently un-removable stage dir is observable rather than silent.
fn cleanup_stage(sidecar_dir: &Path) {
    if let Err(e) = std::fs::remove_dir_all(sidecar_dir)
        && sidecar_dir.exists()
    {
        // Already on an error/exit path or about to return Ok; surface, don't fail.
        ewrite_or_exit(&format!(
            "[BUNDLE] WARN: could not clean staged sidecar {}: {e}",
            sidecar_dir.display()
        ))
        .ok();
    }
}

/// Ad-hoc codesign a whole .app bundle, signing nested binaries first.
fn codesign_deep(app: &Path) -> bool {
    Command::new("codesign")
        .args(["--deep", "--force", "--sign", "-"])
        .arg(app)
        .status()
        .is_ok_and(|s| s.success())
}

/// Force-create a symlink at `link` pointing to `target` (replacing any
/// existing file/symlink). Returns Err(message) on failure.
fn symlink_force(target: &Path, link: &Path) -> Result<(), String> {
    let Some(parent) = link.parent() else {
        return Err(format!("[BUNDLE] FAIL: {} has no parent", link.display()));
    };
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("[BUNDLE] FAIL: mkdir {}: {e}", parent.display()))?;
    // symlink_metadata does not follow the link, so a dangling/old symlink is
    // still detected and removed.
    if link.symlink_metadata().is_ok() {
        std::fs::remove_file(link)
            .map_err(|e| format!("[BUNDLE] FAIL: unlink {}: {e}", link.display()))?;
    }
    platform_symlink(target, link).map_err(|e| {
        format!(
            "[BUNDLE] FAIL: symlink {} -> {}: {e}",
            link.display(),
            target.display()
        )
    })
}

/// Create a file symlink in a platform-portable way. The `--bundle` flow that
/// calls this is macOS-only at runtime, but the function is compiled on every
/// target, so the syscall must resolve on each: `std::os::unix::fs::symlink`
/// on Unix, `std::os::windows::fs::symlink_file` on Windows.
#[cfg(unix)]
fn platform_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn platform_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(target, link)
}

/// Resolve the host target triple via `rustc --print host-tuple`. The bundler
/// appends this to the sidecar filename (Dioxus `external_bin`).
fn host_triple() -> Result<String, String> {
    let out = Command::new("rustc")
        .args(["--print", "host-tuple"])
        .output()
        .map_err(|e| format!("[BUNDLE] FAIL: run rustc --print host-tuple: {e}"))?;
    if !out.status.success() {
        return Err("[BUNDLE] FAIL: rustc --print host-tuple exited non-zero".to_owned());
    }
    let triple = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    if triple.is_empty() {
        return Err("[BUNDLE] FAIL: rustc returned an empty host triple".to_owned());
    }
    Ok(triple)
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

    // A `--bundle` install leaves `dst` as a symlink into KavachApp.app. Plain
    // `install_binary` must REFUSE rather than clobber it into a standalone copy
    // (the divergence that breaks CLI↔GUI/daemon binary identity).
    #[test]
    fn refuses_to_clobber_a_bundle_symlink() {
        let dir = std::env::temp_dir().join(format!("kavach-deploytest-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let src = dir.join("src-bin");
        fs::write(&src, b"#!/bin/sh\n").unwrap();
        let dst = dir.join("kavach");
        // Simulate the bundle symlink target path component.
        let bundle_target = dir
            .join("KavachApp.app")
            .join("Contents")
            .join("MacOS")
            .join("kavach");
        std::os::unix::fs::symlink(&bundle_target, &dst).unwrap();

        let err = install_binary(&src, &dst).expect_err("must refuse bundle symlink clobber");
        assert!(
            err.contains("KavachApp.app"),
            "error must name the bundle: {err}"
        );
        // The symlink must survive untouched.
        assert!(dst.symlink_metadata().unwrap().file_type().is_symlink());

        fs::remove_dir_all(&dir).ok();
    }

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
}
