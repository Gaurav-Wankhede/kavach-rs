// Spawn `claude -p` in an isolated git worktree under ~/.kavach/runs/.
// SOURCE: https://code.claude.com/docs/en/headless
//
// ARCH: SubprocessIsolatedWorktreeRunner
// PATTERN: per_task_worktree
// SCOPE: data
// CAPACITY: ≤2 concurrent runs (enforced at call site)
// QPS: human-driven; 1 run per click
// LATENCY: spawn ~50ms; runtime per task is bounded only by Claude
// CONSISTENCY: each run has its own worktree; no shared mutable state with main checkout
// FAILURE_MODE: spawn fails → returns error message; user sees in UI
// OBSERVABILITY: tracing::info! on spawn, error on failure
// TRADEOFF: disk usage grows per run until user prunes
// SOURCE: https://code.claude.com/docs/en/headless
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

#[derive(Clone, Debug)]
pub struct SpawnRequest {
    pub project_workdir: PathBuf,
    pub branch: String,
    pub worktree_path: PathBuf,
    pub prompt: String,
    pub model_override: Option<String>,
}

pub struct RunningProcess {
    pub child_pid: u32,
    pub events_rx: mpsc::Receiver<String>,
}

/// Create a git worktree, then spawn `claude -p` inside it. Returns the
/// child PID + a channel of stdout lines (one event per line; stream-json
/// produces one JSON object per line).
pub fn spawn(req: &SpawnRequest) -> Result<RunningProcess, String> {
    if let Some(parent) = req.worktree_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir parent: {e}"))?;
    }
    // git worktree add -b <branch> <path>
    let wt = Command::new("git")
        .arg("-C")
        .arg(&req.project_workdir)
        .args(["worktree", "add", "-b"])
        .arg(&req.branch)
        .arg(&req.worktree_path)
        .output()
        .map_err(|e| format!("git worktree spawn: {e}"))?;
    if !wt.status.success() {
        let stderr = String::from_utf8_lossy(&wt.stderr).to_string();
        return Err(format!("git worktree add failed: {stderr}"));
    }

    let mut cmd = Command::new("claude");
    cmd.arg("-p")
        .arg(&req.prompt)
        .args(["--output-format", "stream-json", "--bare"])
        .current_dir(&req.worktree_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(m) = req.model_override.as_ref() {
        cmd.args(["--model", m]);
    }
    let mut child = cmd.spawn().map_err(|e| format!("claude spawn: {e}"))?;
    let pid = child.id();
    let Some(stdout) = child.stdout.take() else {
        return Err(String::from("claude stdout unavailable"));
    };
    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    if tx.send(l).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tx.send(format!("[stdout error] {e}")).ok();
                    break;
                }
            }
        }
        // wait so the zombie reaps; we ignore the exit status here — the UI
        // reads completion from event stream end-of-channel.
        child.wait().ok();
    });
    Ok(RunningProcess {
        child_pid: pid,
        events_rx: rx,
    })
}
