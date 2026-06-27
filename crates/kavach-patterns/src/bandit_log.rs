// split: Layer-A RLVR logging tuple (harness-rl design, Wave P2). One row per gate
// decision: (context x, action a, propensity p, reward r). Pure data + serde; the
// RPC `bandit_log` store persists it (single-writer invariant). NO behavior change.
// WHY this shape (logged-bandit tuple for offline IPS/DR-OPE) + the fail-closed
// reward signs: kavach-db decision.arch.harness-rl.design-2026-06-05.
//
// NOTE: Debug here is SAFE — every field is non-secret telemetry (a session UUID,
// a coarse risk label, byte counts, an enum). No credential/PII is carried. This
// is the rust196-debug false-positive class (pattern.rust196-debug-fp-numeric-budget).

use serde::{Deserialize, Serialize};

/// The action a gate took. Fixed set — the bandit's action space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum GateAction {
    /// Let the tool through.
    Allow,
    /// Defer to the human (the safe abstention action).
    Ask,
    /// Hard-stop the tool.
    Block,
}

/// The downstream reward of a decision, back-filled when the 3-witness lands.
///
/// Signs (fail-closed bias): a false-block — the gate blocked a change the dev
/// later overrode AND it verified clean — is the costly error and scores `-1`,
/// same as a false-allow. A needed `Ask` is neutral. Absent until the witness
/// resolves, so it is `Option` on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Reward {
    /// Verify passed and the card closed clean after an allow.
    VerifiedClean,
    /// An `Ask` that was genuinely needed (neutral).
    NeededAsk,
    /// Allowed then verify failed (false allow), OR blocked a correct change
    /// (false block — dev overrode and it later verified). The costly error.
    FalseDecision,
}

impl Reward {
    /// The scalar value used by the offline OPE estimators (Layer B).
    #[must_use]
    pub const fn value(self) -> i8 {
        match self {
            Self::VerifiedClean => 1,
            Self::NeededAsk => 0,
            Self::FalseDecision => -1,
        }
    }
}

/// The features a gate already sees at decision time — the bandit context `x`.
/// Every field is something the live gate computes; nothing new is collected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct BanditContext {
    /// Which gate decided (e.g. `nano_file_guard`).
    pub gate: String,
    /// The tool under decision (`Bash`, `Write`, `Edit`, `Stop`, ...).
    pub tool: String,
    /// File extension when a path is in scope (`rs`, `ts`, ...), else empty.
    pub file_ext: String,
    /// Size of the change in bytes (diff/content length); 0 when N/A.
    pub diff_bytes: u32,
    /// Classified intent risk (`low`/`medium`/`high`), else empty.
    pub intent_risk: String,
    /// How many times this gate already fired this session (recurrence prior).
    pub prior_fire_count: u32,
}

impl BanditContext {
    /// Construct a decision context. The only way to build one outside this crate
    /// (the struct is `#[non_exhaustive]`), so the emit seam in `kavach-engine`
    /// goes through here.
    #[must_use]
    pub fn new(
        gate: impl Into<String>,
        tool: impl Into<String>,
        file_ext: impl Into<String>,
        diff_bytes: u32,
        intent_risk: impl Into<String>,
        prior_fire_count: u32,
    ) -> Self {
        Self {
            gate: gate.into(),
            tool: tool.into(),
            file_ext: file_ext.into(),
            diff_bytes,
            intent_risk: intent_risk.into(),
            prior_fire_count,
        }
    }
}

/// One logged-bandit row (the RLVR tuple). Append-only; the store assigns the id.
///
/// `propensity` is the CURRENT rule-gate's probability of `action` given the
/// context — `1.0` for a deterministic gate. Logged honestly so Layer B's IPS/DR
/// estimators are unbiased (deterministic logging is weakened — Layer B seeds a
/// small advisory-only randomization band to fix it).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct BanditRow {
    /// The session that produced the decision (provenance + time-split key).
    pub session_id: String,
    /// Wall-clock of the decision (ms since epoch); set by the emitter.
    pub timestamp_ms: i64,
    /// The decision context.
    pub context: BanditContext,
    /// The action the gate took.
    pub action: GateAction,
    /// The logging policy's propensity for `action` in [0, 1].
    pub propensity: f32,
    /// The downstream reward — `None` until the 3-witness back-fills it.
    pub reward: Option<Reward>,
    /// Which reward CHANNEL this row belongs to (P8). `false` (the default for
    /// every pre-P8 row and the on-policy training log) = the HARD, cheap,
    /// code-checkable 3-witness channel. `true` = the SOFT held-out channel: a
    /// sampled slice re-verified independently, the reward-hacking audit's
    /// orthogonal signal (`db.ope_audit` splits the two on exactly this flag). A
    /// held-out row is NEVER fed to policy training — it exists only to catch a
    /// policy that games the cheap witness without earning the real outcome.
    #[serde(default)]
    pub held_out: bool,
}

impl BanditRow {
    /// Construct a freshly-logged decision (reward pending). The emitter passes
    /// `timestamp_ms` so this stays clock-free + deterministic in tests.
    #[must_use]
    pub fn new(
        session_id: impl Into<String>,
        timestamp_ms: i64,
        context: BanditContext,
        action: GateAction,
        propensity: f32,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            timestamp_ms,
            context,
            action,
            propensity: propensity.clamp(0.0, 1.0),
            reward: None,
            held_out: false,
        }
    }

    /// Mark this row as belonging to the SOFT held-out channel (P8).
    ///
    /// Consumes and returns `self` so the emit seam can tag a sampled decision in
    /// one expression: `BanditRow::new(..).into_held_out()`. The audit
    /// (`db.ope_audit`) reads exactly this flag to split the soft re-verification
    /// channel from the hard witness channel; a held-out row is never fed to the
    /// policy-training estimators, only to the reward-hacking drift monitor.
    #[must_use]
    pub const fn into_held_out(mut self) -> Self {
        self.held_out = true;
        self
    }

    /// Whether this row still needs its reward back-filled (Layer-A -> 3-witness).
    #[must_use]
    pub const fn awaits_reward(&self) -> bool {
        self.reward.is_none()
    }
}

#[cfg(test)]
#[path = "bandit_log_test.rs"]
#[cfg(test)]
mod tests;