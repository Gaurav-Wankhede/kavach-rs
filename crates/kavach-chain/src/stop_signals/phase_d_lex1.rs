use super::signal::Signal;
use std::sync::LazyLock;

const UNVERIFIED_CODE_CLAIM_POS: &str = concat!(
    r"(?i)\b(?:I'?ve?\s+(?:written|generated|created|updated)|this\s+(?:code|function|method|implementation))\b[\w\W]{0,80}?\b(?:is|should\s+be)\b",
    r"|\b(?:the\s+)?(?:code|implementation|solution)\b[\w\W]{0,60}?\b(?:is\s+)?(?:ready|complete|done|fixed|working)\b",
    r"|\b(?:this|that)\b[\w\W]{0,60}?\b(?:handles|fixes|solves|addresses)\b[\w\W]{0,40}?\b(?:properly|correctly)\b",
    r"|\b(?:placeholders?|buttons?|handlers?|implementation)[\w\W]{0,60}?\b(?:(?:not|don't)\s+(?:have|exist)|are\s+missing|(?:need|require|need\s+to\s+be))\b",
    r"|\b(?:don't\s+have|missing|not\s+(?:yet\s+|been\s+)?(?:implemented|wired|built))\b",
    r"|\b(?:next\s+phase|backend\s+integration|needs?)\s+(?:of\s+)?work\b",
);

const UNVERIFIED_CODE_CLAIM_NEG: &str = concat!(
    r"(?i)\bI\s+(?:haven't|have\s+not)\s+(?:tested|verified|checked)\b",
    r"|\b(?:tested|verified|checked|examined|grep|found)\b",
    r"|\bcargo\b|\bnextest\b",
    r"|\b(?:I\s+)?(?:checked|read|reviewed|examined)\s+(?:the\s+)?(?:component|file|code|handler)\b",
    r"|\b(?:\w+\.(?:tsx?|jsx?|rs|py):\d+)\b",
    r"|\bsee\s+\w+[\.\w]+:\d+\b",
    r"|```",
);

static UNVERIFIED_CODE_CLAIM: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(UNVERIFIED_CODE_CLAIM_POS)),
    negation: LazyLock::new(|| regex::Regex::new(UNVERIFIED_CODE_CLAIM_NEG)),
};

/// # Errors
/// Returns [`regex::Error`] only if a compile-time-constant pattern fails to
/// compile — unreachable at runtime since the patterns are static literals.
pub fn detect_unverified_code_claim(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    UNVERIFIED_CODE_CLAIM.fires(msg)
}
