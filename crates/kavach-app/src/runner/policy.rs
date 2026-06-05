// Pre-flight checks for spawning a `claude` subprocess.
// SOURCE: https://code.claude.com/docs/en/headless
use std::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreFlight {
    pub claude_path: Option<String>,
    pub api_key_set: bool,
    pub version: Option<String>,
}

pub fn pre_flight() -> PreFlight {
    let claude_path = which("claude");
    let version = claude_path.as_ref().and_then(|p| {
        Command::new(p)
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_owned())
    });
    let api_key_set = std::env::var("ANTHROPIC_API_KEY").is_ok();
    PreFlight {
        claude_path,
        api_key_set,
        version,
    }
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
