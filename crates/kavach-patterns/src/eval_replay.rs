// split: Eval replay framework — re-run current gates against a recorded trajectory.
//
// [RCA]
// symptom:    no way to verify "did the new gate I shipped actually catch the historical bug?"
// repro:      a P0 incident occurs; engineer adds a gate; cannot prove the gate would have caught it
// why1:       no eval/trajectory replay framework
// why2:       hooks emit events but no replayable JSONL canonical format
// why3:       invariant violated — every gate change should be regression-testable against past sessions
// why4:       Anthropic E/V framework: execution-loop checkpointing + trajectory logging is 2026 SOTA
// why5:       missing replay layer
// root_cause: no eval_replay module
// class:      knowledge_gap
// blast_radius: every kavach session
// research:   https://blakecrosley.com/guides/agent-architecture
//             https://www.preprints.org/manuscript/202604.0428
// fix_strategy: pure-Rust replay primitive: TrajectoryEvent JSONL → replay() → Vec<GateOutcome>;
//               wires into all *_guard::detect() so a single replay run exercises every gate

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// One step of an agent trajectory — the smallest replayable unit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate constructed in kavach-engine/gates/stop.rs"
)]
pub struct TrajectoryEvent {
    pub timestamp_ms: i64,
    pub session_id: String,
    pub event_kind: EventKind,
    /// Objective outcome (exit code / test pass-fail), or `None` when no
    /// ground-truth signal exists. `#[serde(default)]` keeps legacy rows valid.
    /// The reward oracle reads THIS, not the agent's prose.
    /// SOURCE: decision.harness-reward-ground-truth-oracle.
    #[serde(default)]
    pub outcome: Option<EventOutcome>,
}

/// Agent-independent outcome attached to a trajectory event — the ground truth
/// the reward oracle scores a success/done claim against. SOURCE above.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum EventOutcome {
    /// The operation objectively succeeded (exit 0 / tests passed).
    Success,
    /// The operation objectively failed (non-zero exit / tests failed / build error).
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum EventKind {
    /// Bash command attempted.
    Bash { command: String },
    /// File write/edit attempted.
    Write { file_path: String, content: String },
    /// Tool other than Bash/Write.
    Tool { name: String, args: String },
    /// Agent stop / claim of done.
    Stop { final_message: String },
}

/// Result of replaying a single event against the gate set.
#[derive(Debug, Clone)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate constructed in replay_event()"
)]
pub struct GateOutcome {
    pub gate: &'static str,
    pub severity: ReplaySeverity,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "cross-crate pattern-matched in replay_event() and tests"
)]
pub enum ReplaySeverity {
    Block,
    Confirm,
    Advise,
    Allow,
}

/// Replay one trajectory event against all gates.
/// Returns the highest-severity outcome and a list of all triggered gates.
fn replay_bash_event(command: &str) -> Vec<GateOutcome> {
    let mut out = Vec::new();
    if let Some(hit) = crate::destructive_cli_guard::inspect(command) {
        use crate::destructive_cli_guard::DestructiveSeverity::{P0Block, P1Confirm, P2Warn};
        let sev = match hit.severity {
            P0Block => ReplaySeverity::Block,
            P1Confirm => ReplaySeverity::Confirm,
            P2Warn => ReplaySeverity::Advise,
        };
        out.push(GateOutcome {
            gate: "destructive_cli_guard",
            severity: sev,
            message: format!("{}: {}", hit.pattern, hit.fix),
        });
    }
    out
}

fn replay_write_event(file_path: &str, content: &str) -> Vec<GateOutcome> {
    let mut out = Vec::new();
    for v in crate::solid_guard::detect(file_path, content) {
        out.push(GateOutcome {
            gate: "solid_guard",
            severity: ReplaySeverity::Advise,
            message: format!("{}: {}", v.pattern, v.fix),
        });
    }
    for v in crate::dsa_guard::detect(file_path, content) {
        out.push(GateOutcome {
            gate: "dsa_guard",
            severity: ReplaySeverity::Advise,
            message: format!("{}: {}", v.pattern, v.fix),
        });
    }
    for v in crate::database_ops_guard::detect(file_path, content) {
        use crate::database_ops_guard::DbOpsSeverity::{P0Block, P1Advisory, P2Warning};
        let sev = match v.severity {
            P0Block => ReplaySeverity::Block,
            P1Advisory | P2Warning => ReplaySeverity::Advise,
        };
        out.push(GateOutcome {
            gate: "database_ops_guard",
            severity: sev,
            message: format!("{}: {}", v.pattern, v.fix),
        });
    }
    for v in crate::pii_data_guard::detect(file_path, content) {
        use crate::pii_data_guard::PiiSeverity::{P0Block, P1Advisory};
        let sev = match v.severity {
            P0Block => ReplaySeverity::Block,
            P1Advisory => ReplaySeverity::Advise,
        };
        out.push(GateOutcome {
            gate: "pii_data_guard",
            severity: sev,
            message: format!("{}: {}", v.pattern, v.fix),
        });
    }
    for v in crate::migration_safety_guard::detect(file_path, content) {
        use crate::migration_safety_guard::MigSeverity::{P0Block, P1Advisory};
        let sev = match v.severity {
            P0Block => ReplaySeverity::Block,
            P1Advisory => ReplaySeverity::Advise,
        };
        out.push(GateOutcome {
            gate: "migration_safety_guard",
            severity: sev,
            message: format!("{}: {}", v.pattern, v.fix),
        });
    }
    for v in crate::webhook_signature_guard::detect(file_path, content) {
        use crate::webhook_signature_guard::WhSeverity::{P0Block, P1Advisory};
        let sev = match v.severity {
            P0Block => ReplaySeverity::Block,
            P1Advisory => ReplaySeverity::Advise,
        };
        out.push(GateOutcome {
            gate: "webhook_signature_guard",
            severity: sev,
            message: format!("{}: {}", v.pattern, v.fix),
        });
    }
    for v in crate::observability_guard::detect(file_path, content) {
        out.push(GateOutcome {
            gate: "observability_guard",
            severity: ReplaySeverity::Advise,
            message: format!("{}: {}", v.pattern, v.fix),
        });
    }
    for v in crate::finops_guard::detect(file_path, content) {
        out.push(GateOutcome {
            gate: "finops_guard",
            severity: ReplaySeverity::Advise,
            message: format!("{}: {}", v.pattern, v.fix),
        });
    }
    for v in crate::axum_guard::detect(file_path, content) {
        out.push(GateOutcome {
            gate: "axum_guard",
            severity: ReplaySeverity::Advise,
            message: format!("{}: {}", v.pattern, v.fix),
        });
    }
    out
}

fn replay_stop_event(final_message: &str) -> Vec<GateOutcome> {
    let mut out = Vec::new();
    if FALSE_COMPLETION
        .as_ref()
        .is_some_and(|re| re.is_match(final_message))
    {
        out.push(GateOutcome {
            gate: "false_completion_detector",
            severity: ReplaySeverity::Confirm,
            message: "Agent claimed completion without test/verify evidence in stop message."
                .into(),
        });
    }
    out
}

#[must_use]
pub fn replay_event(event: &TrajectoryEvent) -> Vec<GateOutcome> {
    match &event.event_kind {
        EventKind::Bash { command } => replay_bash_event(command),
        EventKind::Write { file_path, content } => replay_write_event(file_path, content),
        EventKind::Tool { .. } => vec![],
        EventKind::Stop { final_message } => replay_stop_event(final_message),
    }
}

static FALSE_COMPLETION: LazyLock<Option<Regex>> = LazyLock::new(|| {
    // Claims-of-done without paired evidence verbs (test/verify/build/check).
    Regex::new(r"(?i)\b(?:done|complete|completed|shipped|finished|fixed)\b").ok()
});

/// Replay a full trajectory. Returns one (`event_index`, outcomes) per event.
#[must_use]
pub fn replay_trajectory(events: &[TrajectoryEvent]) -> Vec<(usize, Vec<GateOutcome>)> {
    let mut out = Vec::with_capacity(events.len());
    for (i, ev) in events.iter().enumerate() {
        out.push((i, replay_event(ev)));
    }
    out
}

/// Summary: how many events triggered Block / Confirm / Advise / Allow.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate constructed in summarize()"
)]
pub struct ReplaySummary {
    pub events: usize,
    pub blocks: usize,
    pub confirms: usize,
    pub advises: usize,
    pub allows: usize,
}

#[must_use]
pub fn summarize(events: &[TrajectoryEvent]) -> ReplaySummary {
    let mut s = ReplaySummary {
        events: events.len(),
        ..Default::default()
    };
    for ev in events {
        let outs = replay_event(ev);
        if outs.is_empty() {
            s.allows = s.allows.saturating_add(1);
            continue;
        }
        let worst = outs
            .iter()
            .map(|o| match o.severity {
                ReplaySeverity::Block => 3,
                ReplaySeverity::Confirm => 2,
                ReplaySeverity::Advise => 1,
                ReplaySeverity::Allow => 0,
            })
            .max()
            .unwrap_or(0);
        match worst {
            3 => s.blocks = s.blocks.saturating_add(1),
            2 => s.confirms = s.confirms.saturating_add(1),
            _ => s.advises = s.advises.saturating_add(1),
        }
    }
    s
}

/// Errors produced by the trajectory JSONL emitter/reader.
#[derive(Debug)]
#[expect(
    clippy::exhaustive_enums,
    reason = "cross-crate pattern-matched in emit_to_jsonl() and read_jsonl()"
)]
pub enum EmitError {
    Io(std::io::Error),
    Serde(serde_json::Error),
}

impl std::fmt::Display for EmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io: {e}"),
            Self::Serde(e) => write!(f, "serde: {e}"),
        }
    }
}

impl std::error::Error for EmitError {}

impl From<std::io::Error> for EmitError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for EmitError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serde(e)
    }
}

/// Default trajectory directory: ~/.kavach/trajectories/
/// Stop hooks call this with their session id to append a single line.
///
/// The base resolves from `KAVACH_HOME` if set, else `dirs::home_dir()`. The env
/// override is the only platform-portable test seam: `dirs::home_dir()` reads
/// `$HOME` on Unix but calls the Win32 `SHGetKnownFolderPath` on Windows, so it
/// ignores env vars there — tests cannot redirect it by setting `HOME`/`USERPROFILE`.
/// It also matches the `KAVACH_CONFIG_DIR` convention for operator relocation.
///
/// # Errors
/// Returns `EmitError::Io` if the home directory is not found or directory creation fails.
pub fn default_trajectory_path(session_id: &str) -> Result<std::path::PathBuf, EmitError> {
    let home = std::env::var_os("KAVACH_HOME")
        .map(std::path::PathBuf::from)
        .or_else(dirs::home_dir)
        .ok_or_else(|| {
            EmitError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no home dir",
            ))
        })?;
    let dir = home.join(".kavach").join("trajectories");
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join(format!("{session_id}.jsonl")))
}

/// Append one event as a single JSONL line.
///
/// Single `write_all` for sub-PIPE_BUF atomicity across concurrent hook processes.
/// SOURCE: <https://docs.rs/tokio/latest/tokio/fs/struct.OpenOptions.html> (atomicity note)
///
/// # Errors
/// Returns `EmitError::Io` if file creation/opening fails, or `EmitError::Serde` if serialization fails.
pub fn emit_to_jsonl(path: &std::path::Path, event: &TrajectoryEvent) -> Result<(), EmitError> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut line = serde_json::to_vec(event)?;
    line.push(b'\n');
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)?;
    file.write_all(&line)?;
    Ok(())
}

/// Append one event to a session's default trajectory tape on disk.
///
/// The single capture entry point for the live gates: resolves
/// `default_trajectory_path(session_id)`, builds the `TrajectoryEvent`, and
/// appends it. Caller passes `timestamp_ms` (gates already compute it; keeping
/// `eval_replay` clock-free keeps it deterministic + testable). No-ops on an
/// empty `session_id`. Errors are returned so the caller decides whether to
/// swallow — the hook layer drops them (a tape write must never block a gate).
///
/// # Errors
/// Propagates `EmitError` from path resolution or the JSONL append.
pub fn capture(
    session_id: &str,
    timestamp_ms: i64,
    event_kind: EventKind,
) -> Result<(), EmitError> {
    if session_id.is_empty() {
        return Ok(());
    }
    capture_with_outcome(session_id, timestamp_ms, event_kind, None)
}

/// Capture a trajectory event WITH its objective outcome.
///
/// The outcome is the ground-truth signal (a Bash exit code, a build/test result)
/// the reward oracle scores against. Callers that observe an outcome (the
/// `PostToolUse:Bash` hook has the exit code) pass `Some`; pure self-report events
/// (`Stop`) pass `None`.
///
/// # Errors
/// Propagates `EmitError` from path resolution or the JSONL append.
pub fn capture_with_outcome(
    session_id: &str,
    timestamp_ms: i64,
    event_kind: EventKind,
    outcome: Option<EventOutcome>,
) -> Result<(), EmitError> {
    if session_id.is_empty() {
        return Ok(());
    }
    let path = default_trajectory_path(session_id)?;
    let event = TrajectoryEvent {
        timestamp_ms,
        session_id: session_id.to_owned(),
        event_kind,
        outcome,
    };
    emit_to_jsonl(&path, &event)
}

/// Read a JSONL trajectory back into events.
///
/// Skips malformed lines silently — replay is best-effort; corrupt lines must not block evaluation.
///
/// # Errors
/// Returns `EmitError::Io` if file reading fails, or `EmitError::Serde` if line parsing fails.
pub fn read_jsonl(path: &std::path::Path) -> Result<Vec<TrajectoryEvent>, EmitError> {
    use std::io::{BufRead, BufReader};
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(ev) = serde_json::from_str::<TrajectoryEvent>(&line) {
            out.push(ev);
        }
    }
    Ok(out)
}

#[cfg(test)]
#[path = "eval_replay_test.rs"]
#[cfg(test)]
mod tests;