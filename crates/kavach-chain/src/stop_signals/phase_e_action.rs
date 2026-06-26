//! Phase E — action-driven imperatives: catch a turn that CLAIMS completion,
//! a verdict, a decision, or current-knowledge but did NOT carry the action's
//! proof. Each is the "claims but does not act" failure: prose asserting a result
//! with no artifact. POS = the claim language; NEG = the evidence that the action
//! actually happened (so the signal fires ONLY on claim-without-proof).
use super::signal::Signal;
use std::sync::LazyLock;

// --- completion-without-3-witness -------------------------------------------
// "done / complete / shipped / landed / finished" without the three proofs
// (rg artifact + git diff --stat + cargo/nextest). Distinct from
// unverified_code_claim (code-readiness): this fires on a STATUS/COMPLETION
// claim of any task — the AUTO_CONTINUE narration loophole.
const COMPLETION_POS: &str = concat!(
    r"(?i)\b(?:all\s+)?(?:done|complete[d]?|shipped|landed|finished|wrapped\s+up|task\s+complete)\b",
    r"|\b(?:fully|now)\s+(?:done|complete|working|landed)\b",
);
const COMPLETION_NEG: &str = concat!(
    r"(?i)\bgit\s+diff(?:\s+--stat)?\b|\bcargo\s+(?:check|nextest|test|clippy|build)\b|\bnextest\b",
    r"|\b\w+\.(?:rs|tsx?|jsx?|py|go|sql|toml):\d+\b", // file:line witness
    r"|\b\d+\s+(?:tests?|files?)\s+(?:run|passed|changed)\b", // test/diff witness
    r"|\bexit\s+0\b|\bpassed\b|\+\d+/-\d+\b",
);
static COMPLETION: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(COMPLETION_POS)),
    negation: LazyLock::new(|| regex::Regex::new(COMPLETION_NEG)),
};

/// Fire when the turn declares completion but cites none of the three witnesses
/// (build/diff/artifact). The completion narration IS the loophole.
///
/// # Errors
/// [`regex::Error`] only if a static pattern fails to compile (unreachable).
pub fn detect_completion_without_witnesses(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    COMPLETION.fires(msg)
}

// --- decision-not-persisted -------------------------------------------------
// "decided / chose / going with / the approach is" without a same-turn DB write
// (kavach db write / decision row). A decision in prose evaporates.
const DECISION_POS: &str = concat!(
    r"(?i)\b(?:I'?ve?\s+)?(?:decided|chose|chosen|settled\s+on|going\s+with|the\s+approach\s+is|the\s+design\s+is|we'?ll\s+use)\b",
    r"|\b(?:decision|design\s+choice|trade-?off)\b[\w\W]{0,40}?\b(?:is|=)\b",
);
const DECISION_NEG: &str = concat!(
    r"(?i)\bkavach\s+db\s+write\b|\b--category\s+decision\b|\bdecision\s+row\b|\bpersist(?:ed)?\b[\w\W]{0,30}?\bdb\b",
    r"|\bwrote\s+\[decision\]\b|\bDECISION_MAP\b",
);
static DECISION: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(DECISION_POS)),
    negation: LazyLock::new(|| regex::Regex::new(DECISION_NEG)),
};

/// Fire when the turn announces a settled decision but did not persist it to the
/// kavach DB the same turn.
///
/// # Errors
/// [`regex::Error`] only if a static pattern fails to compile (unreachable).
pub fn detect_decision_not_persisted(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    DECISION.fires(msg)
}

// --- verdict-without-citation -----------------------------------------------
// "clean / wired / safe / no defect / correct" verdict with no file:line read.
const VERDICT_POS: &str = concat!(
    r"(?i)\b(?:looks?|is|are|all)\s+(?:clean|safe|correct|wired(?:\s+up)?|fine|good)\b",
    r"|\b(?:no\s+(?:defects?|bugs?|issues?|problems?)\s+(?:found)?|verified\s+(?:clean|safe))\b",
    r"|\beverything\s+(?:checks?\s+out|is\s+(?:wired|correct))\b",
);
const VERDICT_NEG: &str = concat!(
    r"(?i)\b\w+\.(?:rs|tsx?|jsx?|py|go|sql|toml):\d+\b", // file:line citation
    r"|\bsee\s+\w+[\.\w/]+:\d+\b|\bat\s+\w+[\.\w/]+:\d+\b",
    r"|\[RCA\]|\bline\s+\d+\b",
);
static VERDICT: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(VERDICT_POS)),
    negation: LazyLock::new(|| regex::Regex::new(VERDICT_NEG)),
};

/// Fire when the turn issues a clean/wired/safe verdict without citing the
/// `file:line` it read to reach it.
///
/// # Errors
/// [`regex::Error`] only if a static pattern fails to compile (unreachable).
pub fn detect_verdict_without_citation(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    VERDICT.fires(msg)
}

// --- claim-without-research -------------------------------------------------
// asserts a current-knowledge fact (latest/version/API/pricing/supports) with no
// source URL or [RESEARCH]/SOURCE marker. Weights are stale; the web is truth.
const RESEARCH_POS: &str = concat!(
    r"(?i)\bthe\s+(?:latest|current|newest)\s+(?:version|release)\b",
    r"|\b(?:supports?|requires?|defaults?\s+to|deprecated|removed\s+in|added\s+in)\b[\w\W]{0,40}?\bv?\d+\.\d+\b",
    r"|\bas\s+of\s+(?:the\s+)?(?:latest|current|now)\b",
    r"|\bthe\s+(?:API|crate|library|docs?)\s+(?:says?|states?|requires?|exposes?)\b",
    // syntax/contract claim shape: asserting how a named tool's syntax/flag/method
    // behaves from memory (the SurrealQL/serde class of error). Low-FP: the NEG arm
    // still exempts it when a source URL / --help / docs marker is present.
    r"|\bthe\s+(?:correct|right|proper)\s+(?:syntax|form|flag|idiom|signature)\s+(?:is|for)\b",
    r"|\b(?:SurrealQL|SurrealDB|serde|tokio|axum|clippy|cargo)\b[\w\W]{0,30}?\b(?:syntax|idiom|flag|attribute|method|takes?|expects?)\b",
);
const RESEARCH_NEG: &str = concat!(
    r"(?i)https?://\S+|\[RESEARCH\b|\bSOURCE:\s*\S+|\bRESEARCH:DONE\b",
    r"|\bgithub\.com\b|\bdocs\.rs\b|\bcrates\.io\b|\b--help\b",
);
static RESEARCH: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(RESEARCH_POS)),
    negation: LazyLock::new(|| regex::Regex::new(RESEARCH_NEG)),
};

/// Fire when the turn asserts a current-knowledge fact (version/API/pricing)
/// with no source URL or research marker.
///
/// # Errors
/// [`regex::Error`] only if a static pattern fails to compile (unreachable).
pub fn detect_claim_without_research(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    RESEARCH.fires(msg)
}
