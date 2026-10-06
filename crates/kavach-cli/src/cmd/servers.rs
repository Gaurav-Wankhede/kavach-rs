//! `kavach servers {up,down,status}` — lifecycle for the two background servers
//! the GUI/CLI depend on: the `SurrealDB` store (launchd `ai.shared.kavach-surreal`,
//! ws on `127.0.0.1:7710`) and the HTMX web UI (`kavach web`).
//!
//! The DB server is launchd-owned (`kickstart`/`bootstrap`); the web UI is a
//! short-lived process we (re)spawn on demand and reach over loopback TCP.

use std::process::Command;

use crate::cli::ServersAction;
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};


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

/// Retained for call-site compatibility. Turso is embedded — no server autostart needed.
pub(crate) const fn ensure_db_up() {
    // Turso DB is embedded (libsql in-process). No server autostart needed.
}

pub(crate) fn run(action: &ServersAction) -> i32 {
    match *action {
        ServersAction::Up { port } => up(port),
        ServersAction::Down { port } => down(port),
        ServersAction::Status { port } => status(port),
        ServersAction::Ui { port } => {
            ensure_db_up();
            match kavach_web::serve(port) {
                Ok(()) => 0,
                Err(e) => {
                    if let Err(io_err) = ewrite_or_exit(&format!("kavach web: {e}")) {
                        return into_exit_code(io_err);
                    }
                    1
                }
            }
        }
    }
}

fn port_is_listening(port: u16) -> bool {
    Command::new("lsof")
        .args([format!("-iTCP:{port}"), "-sTCP:LISTEN".to_owned()])
        .output()
        .is_ok_and(|o| o.status.success() && !o.stdout.is_empty())
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
    if let Err(e) = spawn_web(port) {
        return err(&format!("web: {e}"));
    }
    ok(&format!(
        "servers up — turso embedded [UP] · web http://127.0.0.1:{port}"
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
            "web not running on :{port} (turso embedded)"
        ));
    };
    for pid in pids
        .split_whitespace()
        .filter(|p| p.bytes().all(|b| b.is_ascii_digit()))
    {
        drop(Command::new("kill").arg(pid).status());
    }
    ok(&format!(
        "web stopped on :{port} (turso embedded)"
    ))
}

fn status(port: u16) -> i32 {
    let web = if port_is_listening(port) {
        "UP"
    } else {
        "DOWN"
    };
    ok(&format!(
        "turso embedded [UP] · web http://127.0.0.1:{port} [{web}]"
    ))
}
