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
    /// The dispatching session's `KAVACH_SESSION_ID`. A card held by a LIVE lease
    /// of a DIFFERENT session is excluded from selection (multi-session
    /// task-steal fix — two terminals/tools no longer grab the same card).
    /// `None`/empty ⇒ any live-leased card is treated as foreign (fail-closed:
    /// an un-identified session never steals another's active card).
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct NextTaskResult {
    pub key: String,
    pub title: String,
    pub status: String,
    /// Full card body, funneled so the dispatched task is self-contained (context-rot fix); empty for the sentinel/census list.
    #[serde(default)]
    pub content: String,
    /// Opus-authored executor prompt, served alongside the body when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exec_prompt: Option<String>,
}

/// An `in_progress` roadmap card with its full content.
///
/// The `SessionStart` compaction-seam reconcile (E7) needs the body to read the
/// card's `TOUCHES:` expected-paths hint, which the title-only [`NextTaskResult`]
/// cannot carry.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct InProgressCardRow {
    pub key: String,
    pub title: String,
    pub content: String,
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
    /// Dispatching session's id — the lease holder recorded on a winning claim.
    /// `None`/empty (legacy callers) skips the lease and claims status-only, the
    /// pre-lease behaviour. A live caller MUST pass it so a hung holder's card is
    /// protected from foreign resume by a renewable TTL lease, not bare status.
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct ClaimCardResult {
    pub key: String,
    pub status: String,
    pub claimed: bool,
    /// Monotonic fence token from the acquired lease (`occupied_epoch`). `None`
    /// when no lease was taken (legacy status-only claim, or a non-winning
    /// claim). A renewer MUST present this epoch so a stale holder evicted by TTL
    /// cannot push `occupied_until` forward after another session reclaimed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epoch: Option<i64>,
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
/// back (blocked deps / operator-gate). `runnable == 0` → board truly empty;
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
    /// DISPATCH-REACHABLE subset: the runnable/blocked/cyclic counts from the
    /// `roadmap` table for THIS project ONLY — i.e. exactly the set the dispatch
    /// probe (`next_open_task`/`promote_next_backlog`) can actually serve in this
    /// lane. The plain `runnable`/`blocked` above ALSO fold the GLOBAL on-disk
    /// Claude Code `TaskList` store (awareness stamp), which dispatch can NEVER
    /// serve from a project session — so a refuse-stop MUST key off these
    /// roadmap-only fields, never the inflated total, or any project session is
    /// trapped forever whenever the global `TaskList` holds an open item. `#[serde(default)]`
    /// keeps an older daemon's payload (without these fields) deserializing to 0,
    /// which the gate treats as "no dispatch-reachable remainder" → fail-safe.
    #[serde(default)]
    pub roadmap_runnable: usize,
    #[serde(default)]
    pub roadmap_blocked: usize,
    #[serde(default)]
    pub roadmap_cyclic: usize,
}
