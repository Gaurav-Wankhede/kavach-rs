use super::signal::Signal;
use std::sync::LazyLock;

const SELF_REVIEW_STOP_POS: &str = concat!(
    r"(?i)\b(?:I|we)\s+(?:need\s+to|should|must)\s+(?:review|audit|verify|check|inspect)\b",
    r"|\b(?:before\s+(?:shipping|deploying|releasing)|self[\s-]?(?:review|audit))\b",
);

const SELF_REVIEW_STOP_NEG: &str = r"(?i)\bdetector\b";

const UNWIRED_FRONTEND_CLAIM_POS: &str = concat!(
    r"(?i)\b(?:frontend|api\s+(?:client|wiring))\b[\w\W]{0,60}?\b(?:(?:still\s+needs?|requires?|awaits?)\s+wiring|(?:to\s+)?backend|integration|complete)\b",
    r"|\b(?:not\s+(?:yet\s+)?|still)\s+wired\b",
    r"|\bstub[\w\W]{0,30}?\bwill\s+be\s+(?:replaced|wired)\b",
    r"|\b(?:wired|complete)\s+to\s+backend\b",
);

const UNWIRED_FRONTEND_CLAIM_NEG: &str = r"(?i)\bdetector\b|\bstop_detect\b";

static SELF_REVIEW_STOP: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(SELF_REVIEW_STOP_POS)),
    negation: LazyLock::new(|| regex::Regex::new(SELF_REVIEW_STOP_NEG)),
};

static UNWIRED_FRONTEND_CLAIM: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(UNWIRED_FRONTEND_CLAIM_POS)),
    negation: LazyLock::new(|| regex::Regex::new(UNWIRED_FRONTEND_CLAIM_NEG)),
};

/// # Errors
/// Returns [`regex::Error`] only if a compile-time-constant pattern fails to
/// compile — unreachable at runtime since the patterns are static literals.
pub fn detect_self_review_stop(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    SELF_REVIEW_STOP.fires(msg)
}

/// # Errors
/// Returns [`regex::Error`] only if a compile-time-constant pattern fails to
/// compile — unreachable at runtime since the patterns are static literals.
pub fn detect_unwired_frontend_claim(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    UNWIRED_FRONTEND_CLAIM.fires(msg)
}
