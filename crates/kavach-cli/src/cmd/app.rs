// `kavach app` — exec the kavach-app binary so the Dioxus event loop runs in
// its own process. We don't link Dioxus into kavach-cli (would balloon the
// gate-hot binary by 30+ MB). Resolved via PATH; falls back to ~/.local/bin.
// SOURCE: https://dioxuslabs.com/learn/0.7/guides/platforms/
use std::process::Command;

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code};

pub(super) fn run() -> i32 {
    let candidates = [which("kavach-app"), home_local_bin("kavach-app")];
    for path in candidates.iter().flatten() {
        match Command::new(path).status() {
            Ok(s) => {
                // ExitStatus::code() returns Option<i32>:
                // - Some(n) when the process exited normally with code n
                // - None when the process was terminated by a signal (Unix)
                // Returning 0 on signal termination masks abnormal termination.
                // Return 1 (generic error) on signal to ensure visibility.
                return s.code().unwrap_or_else(|| {
                    let msg = "kavach: kavach-app terminated by signal (abnormal exit)";
                    if let Err(io_err) = ewrite_or_exit(msg) {
                        into_exit_code(io_err)
                    } else {
                        1
                    }
                });
            }
            Err(e) => {
                let msg = format!("kavach: failed to spawn {path}: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return into_exit_code(io_err);
                }
            }
        }
    }
    let msg = "kavach: kavach-app binary not found in PATH or ~/.local/bin. Run `cargo build --release -p kavach-app` and copy the binary.";
    if let Err(io_err) = ewrite_or_exit(msg) {
        return into_exit_code(io_err);
    }
    1
}

fn which(name: &str) -> Option<String> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return candidate.to_str().map(str::to_owned);
        }
    }
    None
}

fn home_local_bin(name: &str) -> Option<String> {
    let home = dirs::home_dir()?;
    let candidate = home.join(".local").join("bin").join(name);
    if candidate.is_file() {
        candidate.to_str().map(str::to_owned)
    } else {
        None
    }
}
