use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct EntryStatusParams {
    pub project: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct EntryStatusResult {
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct NextOpenTaskParams {
    pub project: String,
    /// Lane affinity for the dispatching session. `None` (unset / no
    /// `KAVACH_LANE`) behaves exactly as before — the whole project backlog is
    /// the dispatch pool. `Some(lane)` runs a two-pass dispatch: own lane first,
    /// then the unlaned (NULL) backlog, never a foreign lane.
    #[serde(default)]
    pub lane: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct NextTaskResult {
    pub key: String,
    pub title: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct ListTitlesParams {
    pub project: String,
    #[serde(default)]
    pub category: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct TitleRow {
    pub category: String,
    pub key: String,
    pub title: String,
    #[serde(rename = "entry_status")]
    pub entry_status: String,
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct ClaimCardParams {
    pub project: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct ClaimCardResult {
    pub key: String,
    pub status: String,
    pub claimed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct VerifyCardResult {
    pub key: String,
    pub status: String,
    pub verified: bool,
}

/// Open-set census for the stop gate's board-drained branch.
///
/// `runnable` = cards in a dispatchable status; `blocked` = those of them held
/// back (blocked deps / owner-gate). `runnable == 0` → board truly empty;
/// `runnable > 0 && blocked == runnable` → every remainder blocked → clean
/// `[ALL_BLOCKED]` stop.
///
/// `cyclic` = of the runnable set, cards whose declared deps form a dependency
/// cycle (self-dep or mutual). These can NEVER satisfy `deps_satisfied`, so they
/// would otherwise inflate `blocked` and forge a false `[ALL_BLOCKED]` clean-stop
/// while real work sits unreachable. They are counted separately so the gate can
/// refuse the stop and name the deadlock instead of treating it as legitimate.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OpenSetCensus {
    pub runnable: usize,
    pub blocked: usize,
    pub cyclic: usize,
    /// Keys of the runnable cards detected in a dependency cycle (for the gate
    /// message). Empty when `cyclic == 0`.
    pub cyclic_keys: Vec<String>,
}
