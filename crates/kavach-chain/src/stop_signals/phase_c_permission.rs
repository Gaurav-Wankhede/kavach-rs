use super::signal::Signal;
use std::sync::LazyLock;

const PERMISSION_SEEK_POS: &str = concat!(
    r"(?i)\b(?:should\s+I|can\s+I|may\s+I|would\s+you\s+like\s+me\s+to)\s+(?:proceed|continue)\b",
    r"|\b(?:do\s+you\s+want\s+me\s+to|are\s+you\s+okay\s+with\s+me)\s+(?:proceeding|continuing)\b",
    r"|\b(?:is\s+it\s+okay|alright|fine)\s+(?:if\s+I|for\s+me\s+to)\b",
    r"|\b(?:permission|approval|go[\s-]?ahead)\b[\w\W]{0,40}?\b(?:to|for)\b",
    r"|\b(?:choice|decision)\s+(?:rests\s+with|is\s+(?:yours|up\s+to))\s+you\b",
    r"|\b(?:you|your)\s+(?:green[\s-]?light|approval|call)\s+(?:it|the)\b",
);

const PERMISSION_SEEK_NEG: &str =
    r"(?i)\bas\s+you\s+(?:requested|asked|directed)\b|\bper\s+your\s+instruction\b";

static PERMISSION_SEEK: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(PERMISSION_SEEK_POS)),
    negation: LazyLock::new(|| regex::Regex::new(PERMISSION_SEEK_NEG)),
};

/// # Errors
/// Returns [`regex::Error`] only if a compile-time-constant pattern fails to
/// compile — unreachable at runtime since the patterns are static literals.
pub fn detect_permission_seek(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    PERMISSION_SEEK.fires(msg)
}
