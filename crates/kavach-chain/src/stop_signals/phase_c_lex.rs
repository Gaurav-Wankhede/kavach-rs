use super::signal::Signal;
use std::sync::LazyLock;

const DEFERRED_DISMISSAL_POS: &str = concat!(
    r"(?i)\b(?:I\s+)?(?:could|might|may)\s+(?:look|check|investigate)\s+(?:later|next\s+time)\b",
    r"|\b(?:deferred|postponed|left\s+for)\s+(?:later|future|another\s+time)\b",
    r"|\b(?:optional|nice[\s-]?to[\s-]?have|can\s+wait)\b",
    // Blocked/deferred excuse patterns: operator decisions, infrastructure, external dependency
    r"|\b(?:blocked|deferred)[\w\W]{0,80}?\b(?:operator|decision|infrastructure|external|dependency|stable|release)\b",
    // Synonyms: shelved, tabled, parked, sidelined
    r"|\b(?:shelved|tabled|parked|sidelined)[\w\W]{0,60}?\b(?:awaiting|requires?|due\s+to|wait(?:ing)?)\b",
    // Design decision exemption (when no action was taken)
    r"|\b(?:design\s+decision)[\w\W]{0,60}?\b(?:not\s+a\s+bug|blocked|deferred)\b",
    // Markdown table or structured summary with excuse rows
    r"|\|\s*\w+[\w\s]*\s*\|\s*(?:operator|decision|dependency|reason)\b",
    // Uppercase DEFERRED / BLOCKED patterns
    r"|\b(?:DEFERRED|BLOCKED|SIDELINED)[\w\s]*(?:DECISION|DEPENDENCY|ACTION)\b",
);

const DEFERRED_DISMISSAL_NEG: &str = r"(?i)\bdetector\b|\b(?:all|every|shipped|unblocked|verified|all\s+clear)\b[\w\W]{0,40}?\b(?:complete|done|shipped|verified)\b|\b(?:kavach\s+db\s+(?:get|sync))\b";

const USER_REPORT_DISMISSAL_POS: &str = concat!(
    r"(?i)\byou\s+(?:reported|told|mentioned|said)\b[\w\W]{0,40}?\b(?:issue|problem|bug|error|concern)\b",
    r"|\b(?:the\s+user|you)\s+(?:found|reported|said)\b[\w\W]{0,60}?\b(?:is|was)\s+(?:acceptable|fine|okay)\b",
    r"|\b(?:this|it)\s+is\s+(?:expected|correct|fine|working|normal)\s+(?:behavior|as\s+designed)\b",
);

// NEG arm: suppress when the phrase is QUOTED/illustrative or appears in meta-
// discussion ABOUT the gate, not committed as a live dismissal. Without this the
// detector false-fires on its own test fixtures, transcript quotes, and any turn
// that names the banned phrase to describe it (the string-literal-prose FP class:
// kavach_rs_pattern_fix_arch_gate_fp_on_string_literal_prose). The phrase wrapped
// in straight/smart quotes or backticks, or co-occurring with test/gate/transcript/
// regex/detector vocabulary, is meta — never a refutation of the user.
const USER_REPORT_DISMISSAL_NEG: &str = r#"(?i)\bdetector\b|\b(?:root\s+)?cause\s+is\b|\bblocker\s+is\b|["'`“”][^"'`“”]*\b(?:correct|expected|fine|working|normal)\s+(?:behavior|as\s+designed)\b|\b(?:test|fixture|transcript|regex|detector|gate|verbatim|quote[ds]?|example|illustrat)\w*\b"#;

const SUMMARY_EXIT_POS: &str = concat!(
    r"(?i)\b(?:to\s+summarize|in\s+summary|recap|conclusion)\b[\w\W]{0,120}?\b(?:what|next)\b",
    r"|\bsummary[\w\W]{0,60}?\b(?:what|next)\b",
    r"|\btally[\w\W]{0,60}?\b(?:what|next)\b",
    r"|\b(?:that'?s\s+(?:the\s+)?)?(?:all|everything|it)\b[\w\W]{0,40}?\b(?:for\s+this\s+(?:pass|session|turn)|was\s+done)\b",
    r"|\bwe'?re?\s+(?:done|finished|complete)\b[\w\W]{0,30}?\b(?:for\s+(?:now|this\s+session))\b",
    r"|\b(?:bugs?\s+fixed|fixed)[\w\W]{0,40}?\bthis\s+session\b[\w\W]{0,60}?\b(?:what|next)\b",
    r"|\b(?:here'?s?\s+)?(?:what\s+was|what's)\s+done\b[\w\W]{0,60}?\b(?:what|next)\b",
    r"|\b(?:\w+\s+)?commits?\b[\w\W]{0,40}?\bthis\s+session\b[\w\W]{0,60}?\b(?:what|next)\b",
    r"|\btwo\s+commits\b[\w\W]{0,40}?\b(?:fixed|remaining)\b",
    r"|\b(?:what\s+(?:would|should|do)\s+you|what\s+next)\b",
    r"|\b(?:to\s+summarize|in\s+summary)\b",
);

const SUMMARY_EXIT_NEG: &str = r"(?i)\band\s+(?:continuing|still|moving\s+on|starting|implementing)\b|\bnow\s+(?:starting|implementing)\b";

static DEFERRED_DISMISSAL: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(DEFERRED_DISMISSAL_POS)),
    negation: LazyLock::new(|| regex::Regex::new(DEFERRED_DISMISSAL_NEG)),
};

static USER_REPORT_DISMISSAL: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(USER_REPORT_DISMISSAL_POS)),
    negation: LazyLock::new(|| regex::Regex::new(USER_REPORT_DISMISSAL_NEG)),
};

static SUMMARY_EXIT: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(SUMMARY_EXIT_POS)),
    negation: LazyLock::new(|| regex::Regex::new(SUMMARY_EXIT_NEG)),
};

/// # Errors
/// Returns [`regex::Error`] only if a compile-time-constant pattern fails to
/// compile — unreachable at runtime since the patterns are static literals.
pub fn detect_deferred_dismissal(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    DEFERRED_DISMISSAL.fires(msg)
}

/// # Errors
/// Returns [`regex::Error`] only if a compile-time-constant pattern fails to
/// compile — unreachable at runtime since the patterns are static literals.
pub fn detect_user_report_dismissal(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    USER_REPORT_DISMISSAL.fires(msg)
}

/// # Errors
/// Returns [`regex::Error`] only if a compile-time-constant pattern fails to
/// compile — unreachable at runtime since the patterns are static literals.
pub fn detect_summary_exit(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    SUMMARY_EXIT.fires(msg)
}
