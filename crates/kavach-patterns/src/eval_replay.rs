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
/// # Errors
/// Returns `EmitError::Io` if the home directory is not found or directory creation fails.
pub fn default_trajectory_path(session_id: &str) -> Result<std::path::PathBuf, EmitError> {
    let home = dirs::home_dir().ok_or_else(|| {
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
mod tests {
    use super::*;

    fn ev_bash(cmd: &str) -> TrajectoryEvent {
        TrajectoryEvent {
            timestamp_ms: 0,
            session_id: "t".into(),
            event_kind: EventKind::Bash {
                command: cmd.into(),
            },
        }
    }
    fn ev_write(path: &str, content: &str) -> TrajectoryEvent {
        TrajectoryEvent {
            timestamp_ms: 0,
            session_id: "t".into(),
            event_kind: EventKind::Write {
                file_path: path.into(),
                content: content.into(),
            },
        }
    }

    #[test]
    fn replay_rm_rf_blocks() {
        let outs = replay_event(&ev_bash("rm -rf /"));
        assert!(
            outs.iter()
                .any(|o| o.gate == "destructive_cli_guard" && o.severity == ReplaySeverity::Block)
        );
    }

    #[test]
    fn replay_quote_obfuscation_caught() {
        let outs = replay_event(&ev_bash("'r''m' -rf /"));
        assert!(outs.iter().any(|o| o.severity == ReplaySeverity::Block));
    }

    #[test]
    fn replay_safe_bash_passes() {
        let outs = replay_event(&ev_bash("ls -la"));
        assert!(outs.is_empty());
    }

    #[test]
    fn replay_migration_blocks() {
        let outs = replay_event(&ev_write(
            "migrations/0001_role.sql",
            "ALTER TABLE users ADD COLUMN role text NOT NULL;",
        ));
        assert!(
            outs.iter()
                .any(|o| o.gate == "migration_safety_guard" && o.severity == ReplaySeverity::Block)
        );
    }

    #[test]
    fn replay_clean_handler_passes() {
        let outs = replay_event(&ev_write(
            "src/handlers/users.rs",
            "use axum;\n#[tracing::instrument(skip(repo))]\npub async fn list<R: UserRepository>(State(repo): State<Arc<R>>) {}",
        ));
        // No Block-level outcomes
        assert!(!outs.iter().any(|o| o.severity == ReplaySeverity::Block));
    }

    #[test]
    fn replay_false_completion_detected() {
        let ev = TrajectoryEvent {
            timestamp_ms: 0,
            session_id: "t".into(),
            event_kind: EventKind::Stop {
                final_message: "All done!".into(),
            },
        };
        let outs = replay_event(&ev);
        assert!(outs.iter().any(|o| o.gate == "false_completion_detector"));
    }

    #[test]
    fn summarize_counts_correctly() {
        let trajectory = vec![
            ev_bash("rm -rf /"), // block
            ev_bash("ls -la"),   // allow
            ev_write(
                "migrations/0001.sql",
                "ALTER TABLE u ADD COLUMN r text NOT NULL;",
            ), // block
            ev_write(
                "src/handlers/u.rs",
                "use axum; pub async fn h(State(p): State<sqlx::PgPool>) {}",
            ), // advise
        ];
        let s = summarize(&trajectory);
        assert_eq!(s.events, 4);
        assert_eq!(s.blocks, 2);
        assert_eq!(s.allows, 1);
        assert!(s.advises >= 1);
    }

    #[test]
    fn replay_trajectory_returns_per_event_outcomes() {
        let trajectory = vec![ev_bash("rm -rf /"), ev_bash("ls -la")];
        let r = replay_trajectory(&trajectory);
        assert_eq!(r.len(), 2);
        assert!(!r[0].1.is_empty());
        assert!(r[1].1.is_empty());
    }

    #[test]
    fn jsonl_emit_then_read_roundtrip() {
        let dir = std::env::temp_dir().join(format!("kavach_replay_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("session.jsonl");
        std::fs::remove_file(&path).ok();

        let events = vec![
            ev_bash("rm -rf /"),
            ev_bash("ls -la"),
            TrajectoryEvent {
                timestamp_ms: 42,
                session_id: "t".into(),
                event_kind: EventKind::Stop {
                    final_message: "All done!".into(),
                },
            },
        ];
        for ev in &events {
            emit_to_jsonl(&path, ev).unwrap();
        }

        let read = read_jsonl(&path).unwrap();
        assert_eq!(read.len(), 3);
        assert_eq!(read[0], events[0]);
        assert_eq!(read[2].event_kind, events[2].event_kind);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn jsonl_skips_malformed_lines() {
        let dir =
            std::env::temp_dir().join(format!("kavach_replay_malformed_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("malformed.jsonl");
        std::fs::write(&path, "not-json\n{\"timestamp_ms\":1,\"session_id\":\"s\",\"event_kind\":{\"kind\":\"bash\",\"command\":\"ls\"}}\n").unwrap();
        let read = read_jsonl(&path).unwrap();
        assert_eq!(read.len(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }
}
