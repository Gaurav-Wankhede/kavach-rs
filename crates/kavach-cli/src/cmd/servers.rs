//! `kavach servers {up,down,status}` — lifecycle for the two background servers
//! the GUI/CLI depend on: the `SurrealDB` store (launchd `ai.shared.kavach-surreal`,
//! ws on `127.0.0.1:7710`) and the HTMX web UI (`kavach web`).
//!
//! The DB server is launchd-owned (`kickstart`/`bootstrap`); the web UI is a
//! short-lived process we (re)spawn on demand and reach over loopback TCP.

use std::process::Command;

use crate::cli::ServersAction;
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

const SURREAL_LABEL: &str = "ai.shared.kavach-surreal";
const SURREAL_PORT: u16 = 7710;

fn ok(msg: &str) -> i32 {
    match print_or_exit(msg) {
        Ok(()) => 0,
        Err(io) => into_exit_code(io),
    }
}

fn err(msg: &str) -> i32 {
    match ewrite_or_exit(&format!("error: {msg}")) {
        Ok(()) => 1,
        Err(io) => into_exit_code(io),
    }
}

/// Best-effort ensure the `SurrealDB` server is running (called before serving
/// the web UI). Silent — the offline panel still covers a genuine failure.
pub(crate) fn ensure_db_up() {
    if let Err(e) = ensure_surreal() {
        drop(ewrite_or_exit(&format!(
            "warning: surreal autostart failed: {e}"
        )));
    }
}

pub(crate) fn run(action: &ServersAction) -> i32 {
    match *action {
        ServersAction::Up { port } => up(port),
        ServersAction::Down { port } => down(port),
        ServersAction::Status { port } => status(port),
    }
}

fn uid() -> Option<String> {
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
}

fn port_is_listening(port: u16) -> bool {
    Command::new("lsof")
        .args([format!("-iTCP:{port}"), "-sTCP:LISTEN".to_owned()])
        .output()
        .is_ok_and(|o| o.status.success() && !o.stdout.is_empty())
}

fn ensure_surreal() -> Result<bool, String> {
    if port_is_listening(SURREAL_PORT) {
        return Ok(true);
    }
    let uid = uid().ok_or_else(|| "cannot resolve uid".to_owned())?;
    let domain = format!("gui/{uid}");
    let service = format!("{domain}/{SURREAL_LABEL}");
    let kicked = Command::new("launchctl")
        .args(["kickstart", "-k", &service])
        .status()
        .is_ok_and(|s| s.success());
    if kicked {
        return Ok(true);
    }
    let home = std::env::var("HOME").map_err(|_| "HOME unset".to_owned())?;
    let plist = format!("{home}/Library/LaunchAgents/{SURREAL_LABEL}.plist");
    if !std::path::Path::new(&plist).exists() {
        return Err(format!(
            "surreal launchd plist not installed at {plist} — install it first"
        ));
    }
    let booted = Command::new("launchctl")
        .args(["bootstrap", &domain, &plist])
        .status()
        .is_ok_and(|s| s.success());
    booted
        .then_some(true)
        .ok_or_else(|| "launchctl bootstrap failed".to_owned())
}

fn web_self_exe() -> Option<std::path::PathBuf> {
    let me = std::env::current_exe().ok()?;
    let sibling = me.parent()?.join("kavach-web");
    sibling.exists().then_some(sibling)
}

fn spawn_web(port: u16) -> Result<(), String> {
    if port_is_listening(port) {
        return Ok(());
    }
    let exe =
        web_self_exe().ok_or_else(|| "kavach-web binary not found next to kavach".to_owned())?;
    Command::new(exe)
        .arg(port.to_string())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("spawn kavach-web: {e}"))?;
    for _ in 0..40 {
        if port_is_listening(port) {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    Err(format!("kavach-web did not bind :{port} within 2s"))
}

fn up(port: u16) -> i32 {
    if let Err(e) = ensure_surreal() {
        return err(&format!("surreal: {e}"));
    }
    if let Err(e) = spawn_web(port) {
        return err(&format!("web: {e}"));
    }
    ok(&format!(
        "servers up — surreal ws://127.0.0.1:{SURREAL_PORT} · web http://127.0.0.1:{port}"
    ))
}

fn down(port: u16) -> i32 {
    let killed = Command::new("lsof")
        .args([format!("-tiTCP:{port}"), "-sTCP:LISTEN".to_owned()])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .filter(|s| !s.is_empty());
    let Some(pids) = killed else {
        return ok(&format!(
            "web not running on :{port} (surreal left up — launchd-owned)"
        ));
    };
    for pid in pids
        .split_whitespace()
        .filter(|p| p.bytes().all(|b| b.is_ascii_digit()))
    {
        drop(Command::new("kill").arg(pid).status());
    }
    ok(&format!(
        "web stopped on :{port} (surreal left up — bootout it via launchctl if intended)"
    ))
}

fn status(port: u16) -> i32 {
    let surreal = if port_is_listening(SURREAL_PORT) {
        "UP"
    } else {
        "DOWN"
    };
    let web = if port_is_listening(port) {
        "UP"
    } else {
        "DOWN"
    };
    ok(&format!(
        "surreal ws://127.0.0.1:{SURREAL_PORT} [{surreal}] · web http://127.0.0.1:{port} [{web}]"
    ))
}
