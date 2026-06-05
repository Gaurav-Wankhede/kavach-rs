//! launchd-owned RPC daemon respawn (kickstart, else bootstrap from plist).
//!
//! kavach-engine is runtime-free; daemon lifecycle is owned by a launchd
//! `LaunchAgent` (`ai.shared.kavach-rpc`, KeepAlive=true), NOT an in-hook orphan
//! (which is reaped with the hook's process group on return).

/// One-shot RPC daemon recovery via launchd. Returns true if a recovery action
/// was issued. `launchctl kickstart -k` force-restarts the bootstrapped agent;
/// if not yet bootstrapped, `bootstrap` from the installed plist.
#[cfg(unix)]
pub(super) fn try_spawn_rpc_daemon() -> bool {
    use std::process::Command;
    const LABEL: &str = "ai.shared.kavach-rpc";
    // `gui/<uid>` is the launchd domain target. Resolve uid via `id -u` — no
    // libc/nix dep (kavach-engine is intentionally minimal), no unsafe FFI.
    let uid = Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()));
    let Some(uid) = uid else {
        return false;
    };
    let domain = format!("gui/{uid}");
    let service = format!("{domain}/{LABEL}");

    // Force-restart the launchd-owned agent. -k terminates the current instance
    // first; KeepAlive + this kickstart bring it straight back.
    let kicked = Command::new("launchctl")
        .args(["kickstart", "-k", &service])
        .status()
        .is_ok_and(|s| s.success());
    if kicked {
        return true;
    }

    // Agent not bootstrapped yet — bootstrap from the installed plist.
    let Ok(home) = std::env::var("HOME") else {
        return false;
    };
    let plist = format!("{home}/Library/LaunchAgents/{LABEL}.plist");
    if !std::path::Path::new(&plist).exists() {
        return false;
    }
    Command::new("launchctl")
        .args(["bootstrap", &domain, &plist])
        .status()
        .is_ok_and(|s| s.success())
}

#[cfg(not(unix))]
pub(super) const fn try_spawn_rpc_daemon() -> bool {
    false
}
