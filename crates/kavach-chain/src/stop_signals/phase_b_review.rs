use super::signal::Signal;
use std::sync::LazyLock;

const SELF_REVIEW_STOP_POS: &str = concat!(
    r"(?i)\b(?:I|we)\s+(?:haven't|have\s+not|should)\s+(?:review|verify|check|audit|inspect)\b",
    r"|\b(?:needs?|requires?|demands?)\s+(?:a\s+)?(?:review|audit|check|inspection)\b",
    r"|\b(?:before|prior\s+to)\s+(?:shipping|deploying|releasing|committing)\b",
    r"|\bself[\s-]?(?:review|audit|check)\b",
    r"|\b(?:I\s+)?(?:should|must|need\s+to)\s+review\b",
);

const SELF_REVIEW_STOP_NEG: &str = r"(?i)\b(?:already\s+(?:reviewed|audited|checked)|completed|done)\b";

static SELF_REVIEW_STOP: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(SELF_REVIEW_STOP_POS)),
    negation: LazyLock::new(|| regex::Regex::new(SELF_REVIEW_STOP_NEG)),
};

pub fn detect_self_review_stop(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    SELF_REVIEW_STOP.fires(msg)
}
