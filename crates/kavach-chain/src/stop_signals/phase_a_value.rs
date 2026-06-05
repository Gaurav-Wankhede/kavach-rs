use super::signal::Signal;
use std::sync::LazyLock;

const VALUE_GATING_POS: &str = concat!(
    r"(?i)\b(?:adds?\s+)?(?:zero|no|minimal|low|little|marginal)\s+(?:value|benefit|roi)\b",
    r"|\bwould\s+just\s+(?:be|show)\s+(?:empty|noise|a\s+single|nothing)\b",
    r"|\b(?:sufficient|enough|good\s+enough|fine)\s+(?:for\s+(?:launch|now|mvp)|as[\s-]?is)\b",
    r"|\b(?:once|until|when)\s+(?:there\s+(?:are|is|'s)|you\s+have|we\s+have)\b",
    r"|\bnot\s+worth\s+(?:building|adding|doing)\s+(?:until|yet)\b",
);

const VALUE_GATING_NEG: &str = concat!(
    r"(?i)\bbut\s+(?:building|i'?ll\s+build|implementing)\b",
    r"|\b(?:building|implementing)\s+(?:it\s+)?anyway\b",
);

static VALUE_GATING: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(VALUE_GATING_POS)),
    negation: LazyLock::new(|| regex::Regex::new(VALUE_GATING_NEG)),
};

/// # Errors
/// Returns [`regex::Error`] only if a compile-time-constant pattern fails to
/// compile — unreachable at runtime since the patterns are static literals.
pub fn detect_value_gating(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    VALUE_GATING.fires(msg)
}
