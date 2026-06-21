// split: intentional - cohesive replay RPC group (event + trajectory + summarize)
// JSON-RPC method handlers exposing kavach_patterns::eval_replay over the existing socket.
// SOURCE: https://docs.rs/jsonrpsee/latest/jsonrpsee/struct.RpcModule.html
use jsonrpsee::types::ErrorObjectOwned;
use kavach_patterns::eval_replay::{
    self, EventKind, EventOutcome, GateOutcome, ReplaySeverity, ReplaySummary, TrajectoryEvent,
};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

/// Cap on events per replay call. Shields the server from `DoS` via attacker-shaped
/// trajectories (each event triggers regex set passes across every guard).
const MAX_REPLAY_EVENTS: usize = 10_000;

fn invalid(msg: impl Into<String>) -> ErrorObjectOwned {
    ErrorObjectOwned::owned(-32602, msg.into(), None::<()>)
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum WireEventKind {
    Bash { command: String },
    Write { file_path: String, content: String },
    Tool { name: String, args: String },
    Stop { final_message: String },
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct WireEvent {
    pub timestamp_ms: i64,
    pub session_id: String,
    pub event: WireEventKind,
    /// Optional objective outcome (the ground-truth signal the reward oracle
    /// scores against). Absent on legacy/self-report-only wire events.
    #[serde(default)]
    pub outcome: Option<EventOutcome>,
}

impl From<WireEvent> for TrajectoryEvent {
    fn from(w: WireEvent) -> Self {
        let event_kind = match w.event {
            WireEventKind::Bash { command } => EventKind::Bash { command },
            WireEventKind::Write { file_path, content } => EventKind::Write { file_path, content },
            WireEventKind::Tool { name, args } => EventKind::Tool { name, args },
            WireEventKind::Stop { final_message } => EventKind::Stop { final_message },
        };
        Self {
            timestamp_ms: w.timestamp_ms,
            session_id: w.session_id,
            event_kind,
            outcome: w.outcome,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct WireOutcome {
    pub gate: String,
    pub severity: String,
    pub message: String,
}

const fn wire_severity(s: ReplaySeverity) -> &'static str {
    match s {
        ReplaySeverity::Block => "block",
        ReplaySeverity::Confirm => "confirm",
        ReplaySeverity::Advise => "advise",
        ReplaySeverity::Allow => "allow",
    }
}

fn to_wire(o: GateOutcome) -> WireOutcome {
    WireOutcome {
        gate: o.gate.into(),
        severity: wire_severity(o.severity).into(),
        message: o.message,
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct ReplayEventParams {
    pub event: WireEvent,
}

/// Replays a single event through the gate evaluation pipeline.
///
/// # Errors
/// Never returns an error; returns empty outcomes if evaluation completes.
pub fn replay_event(
    _state: &AppState,
    params: ReplayEventParams,
) -> Result<Vec<WireOutcome>, ErrorObjectOwned> {
    let ev: TrajectoryEvent = params.event.into();
    Ok(eval_replay::replay_event(&ev)
        .into_iter()
        .map(to_wire)
        .collect())
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct ReplayTrajectoryParams {
    pub events: Vec<WireEvent>,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct WireTrajectoryStep {
    pub index: usize,
    pub outcomes: Vec<WireOutcome>,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct WireSummary {
    pub events: usize,
    pub blocks: usize,
    pub confirms: usize,
    pub advises: usize,
    pub allows: usize,
}

impl From<ReplaySummary> for WireSummary {
    fn from(s: ReplaySummary) -> Self {
        Self {
            events: s.events,
            blocks: s.blocks,
            confirms: s.confirms,
            advises: s.advises,
            allows: s.allows,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct ReplayTrajectoryResult {
    pub steps: Vec<WireTrajectoryStep>,
    pub summary: WireSummary,
}

/// Replays a trajectory (sequence of events) and returns per-step outcomes.
///
/// # Errors
/// Returns an error if the event count exceeds `MAX_REPLAY_EVENTS`.
pub fn replay_trajectory(
    _state: &AppState,
    params: ReplayTrajectoryParams,
) -> Result<ReplayTrajectoryResult, ErrorObjectOwned> {
    if params.events.len() > MAX_REPLAY_EVENTS {
        return Err(invalid(format!("events exceeds {MAX_REPLAY_EVENTS} cap")));
    }
    let events: Vec<TrajectoryEvent> = params.events.into_iter().map(Into::into).collect();
    let raw: Vec<(usize, Vec<GateOutcome>)> = eval_replay::replay_trajectory(&events);
    let steps = raw
        .into_iter()
        .map(|(index, outs)| WireTrajectoryStep {
            index,
            outcomes: outs.into_iter().map(to_wire).collect(),
        })
        .collect();
    let summary = eval_replay::summarize(&events).into();
    Ok(ReplayTrajectoryResult { steps, summary })
}
