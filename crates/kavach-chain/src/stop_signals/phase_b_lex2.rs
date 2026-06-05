use super::signal::Signal;
use std::sync::LazyLock;

const REMAINING_PHASES_POS: &str = concat!(
    r"(?i)\b(?:remaining|next|remaining\s+phase)[\w\W]{0,60}?(?:is|:)",
    r"|\b(?:phase\s+\d+|step\s+\d+)[\w\W]{0,40}?\b(?:awaits|is|pending|remains)\b",
    r"|\b(?:remaining\s+(?:bugs?|issues?)|issues?\s+remaining)\b",
);

const REMAINING_PHASES_NEG: &str =
    r"(?i)\bas\s+(?:per|part\s+of)\s+the\s+plan\b|\bnow\s+(?:implementing|starting)\b";

const PARALLEL_SYSTEM_POS: &str = concat!(
    r"(?i)\b(?:in\s+parallel|concurrently|simultaneously)\b[\w\W]{0,80}?\b(?:system|other|task)\b",
    r"|\b(?:multiple\s+track|dual\s+system|fork|branching)\b",
    r"|\b(?:create|build|add|adding)\s+(?:a\s+)?(?:new|separate|unified)\s+(?:crate|package|manager|system)\b",
    r"|\b(?:new|separate|unified)\s+(?:crate|package|module|manager|system)[\w\W]{0,40}?\b(?:for|alongside|with)\b",
    r"|\b(?:alongside|concurrently)[\w\W]{0,40}?\b(?:existing|other|legacy)\s+(?:code|system)\b",
);

const PARALLEL_SYSTEM_NEG: &str = r"(?i)\bdetector\b|\breplace\b";

const PASSIVE_INFO_REQUEST_POS: &str = concat!(
    r"(?i)\b(?:let\s+me\s+know|tell\s+me|inform\s+me|update\s+me)\b[\w\W]{0,60}?\b(?:when|if|once|and\s+I'?ll)\b",
    r"|\b(?:can\s+)?you\s+(?:provide|share|tell\s+me|give\s+me|send|check|look)\s+(?:info|details|update|status|(?:\w+\s+)?output)\b",
    r"|\b(?:can\s+)?you\s+(?:share|provide|tell\s+me)[\w\W]{0,40}?\band\s+I'?ll\b",
    r"|\b(?:status|update|progress|result)[\w\W]{0,40}?\b(?:when\s+)?(?:ready|done|complete)\b",
    r"|\b(?:share|provide)[\w\W]{0,40}?\band\s+I'?ll[\w\W]{0,40}?\b(?:give|generate|configure|update)\b",
    r"|\b(?:check|look)\s+(?:with|in|your)[\w\W]{0,40}?\b(?:whoami|dashboard|output)\b",
    r"|\b(?:paste|run[\w\s]+and\s+paste)[\w\W]{0,40}?\b(?:here|output)\b",
);

const PASSIVE_INFO_REQUEST_NEG: &str = r"(?i)\bdetector\b";

static REMAINING_PHASES: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(REMAINING_PHASES_POS)),
    negation: LazyLock::new(|| regex::Regex::new(REMAINING_PHASES_NEG)),
};

static PARALLEL_SYSTEM: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(PARALLEL_SYSTEM_POS)),
    negation: LazyLock::new(|| regex::Regex::new(PARALLEL_SYSTEM_NEG)),
};

static PASSIVE_INFO_REQUEST: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(PASSIVE_INFO_REQUEST_POS)),
    negation: LazyLock::new(|| regex::Regex::new(PASSIVE_INFO_REQUEST_NEG)),
};

/// # Errors
/// Returns [`regex::Error`] only if a compile-time-constant pattern fails to
/// compile — unreachable at runtime since the patterns are static literals.
pub fn detect_remaining_phases(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    REMAINING_PHASES.fires(msg)
}

/// # Errors
/// Returns [`regex::Error`] only if a compile-time-constant pattern fails to
/// compile — unreachable at runtime since the patterns are static literals.
pub fn detect_parallel_system(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    PARALLEL_SYSTEM.fires(msg)
}

/// # Errors
/// Returns [`regex::Error`] only if a compile-time-constant pattern fails to
/// compile — unreachable at runtime since the patterns are static literals.
pub fn detect_passive_info_request(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    PASSIVE_INFO_REQUEST.fires(msg)
}
