use regex::Regex;
use std::sync::LazyLock;

const ANALYSIS_SIGNAL: &str = concat!(
    r"(?i)\bhere'?s\s+what(?:'?s|\s+is)\s+happening\b",
    r"|\bthe\s+(?:issue|problem|bug|error|failure|reason|flow|sequence|callback)\s+is\b",
    r"|\b(?:the\s+)?root\s+cause(?:\s+is)?\b",
    r"|\bthis\s+(?:is\s+because|happens\s+because|is\s+why)\b",
    r"|\breason\s+for\s+this\b|\b(?:now\s+i|i)\s+understand\s+the\b",
);

const FIX_SIGNAL: &str = concat!(
    r"(?i)\b(?:fixed|implemented|wrote|created|added)\b",
    r"|\b(?:updated|modified|changed)\s+the\b",
    r"|\bhere'?s\s+the\s+fix\b|\b(?:applying|fix)\s+(?:the\s+fix|applied)\b",
    r"|\bimplement(?:ing)?\b(?:\s+\w+){0,3}?\s+(?:fix|now)\b",
    r"|\b(?:now\s+implementing|implementing\s+now|fixing\s+now|will\s+fix)\b",
);

const ENDS_QUESTION: &str = concat!(
    r"(?i)\bis\s+this\s+the\s+missing\s+piece\b",
    r"|\b(?:want\s+me\s+to|should\s+i|shall\s+i|would\s+you\s+like|do\s+you\s+want)\b",
);

static ANALYSIS_RE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(ANALYSIS_SIGNAL));
static FIX_RE: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| Regex::new(FIX_SIGNAL));
static ENDS_Q_RE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(ENDS_QUESTION));

/// # Errors
/// Returns [`regex::Error`] only if a compile-time-constant pattern fails to
/// compile — unreachable at runtime since the patterns are static literals.
pub fn detect_research_only_stop(msg: &str, had_write_tool: bool) -> Result<bool, regex::Error> {
    if msg.is_empty() || had_write_tool {
        return Ok(false);
    }
    let analysis = ANALYSIS_RE.as_ref().map_err(Clone::clone)?;
    if !analysis.is_match(msg) {
        return Ok(false);
    }
    let has_fix = FIX_RE.as_ref().map_err(Clone::clone)?.is_match(msg);
    let ends_q = ENDS_Q_RE.as_ref().map_err(Clone::clone)?.is_match(msg);
    Ok(!has_fix || ends_q)
}
