use super::signal::Signal;
use std::sync::LazyLock;

const SELF_LIMIT_POS: &str = concat!(
    r"(?i)\b(?:would\s+)?degrade\s+quality\b|\bquality\s+(?:would\s+suffer|degradation|concerns?)\b",
    r"|\b(?:single|this)\s+(?:autonomous\s+)?turn\b",
    r"|\bhours\s+of\s+(?:focused\s+)?work\b",
    r"|\b(?:token|context)\s+(?:consumption|window|limit|budget)\b",
    r"|\brunning\s+low\s+on\s+context\b",
    r"|\bready\s+for\s+(?:the\s+)?next\s+session\b",
    r"|\b(?:honest|my|capacity)\s+limit\b",
    r"|\bto\s+avoid\s+overwhelming\b|\bmanageable\s+chunks\b",
    r"|\b(?:splitting|breaking)\s+(?:this\s+)?(?:across|into)\s+sessions\b",
);

const SELF_LIMIT_NEG: &str = concat!(
    r"(?i)\bdetector\b|\bgate\s+catches\b|\bstop_detect\b",
    r"|\bas\s+you\s+requested\b|\bper\s+your\s+instruction\b",
    r"|\byou\s+asked\s+(?:me\s+)?to\s+stop\b",
);

const REPRIORITIZE_POS: &str = concat!(
    r"(?i)\bthe\s+priority\s+(?:right\s+now\s+)?(?:is|should\s+be)\b",
    r"|\bmore\s+important\s+right\s+now\b",
    r"|\b(?:better\s+to|should|instead,?)\s+focus\s+on\b",
    r"|\bfocus\s+should\s+be\s+on\b",
    r"|\b(?:better\s+spent|time\s+better\s+spent)\s+on\b",
);

const REPRIORITIZE_NEG: &str =
    r"(?i)\bas\s+you\s+requested\b|\bper\s+your\s+request\b|\bas\s+asked\b";

static SELF_LIMIT: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(SELF_LIMIT_POS)),
    negation: LazyLock::new(|| regex::Regex::new(SELF_LIMIT_NEG)),
};

static REPRIORITIZE: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(REPRIORITIZE_POS)),
    negation: LazyLock::new(|| regex::Regex::new(REPRIORITIZE_NEG)),
};

/// # Errors
/// Returns [`regex::Error`] only if a compile-time-constant pattern fails to
/// compile — unreachable at runtime since the patterns are static literals.
pub fn detect_self_imposed_limit(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    SELF_LIMIT.fires(msg)
}

/// # Errors
/// Returns [`regex::Error`] only if a compile-time-constant pattern fails to
/// compile — unreachable at runtime since the patterns are static literals.
pub fn detect_unsolicited_reprioritization(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    REPRIORITIZE.fires(msg)
}
