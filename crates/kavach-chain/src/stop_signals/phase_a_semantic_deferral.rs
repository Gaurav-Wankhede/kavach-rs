use super::phase_a_deferral::detect_strategic_deferral;
use std::sync::LazyLock;

const HANDOFF_PARAPHRASE: &str = concat!(
    r"(?i)\b(?:hand(?:ed|ing)?|pass(?:ed|ing)?|leav(?:e|ing)|left)\b",
    r"\W+(?:\w+\W+){0,4}?\b(?:remainder|rest|residual|continuation|follow[\s-]?up|the\s+next\s+\w+)\b",
    r"|\b(?:natural|good|sensible|reasonable|clean)\s+(?:stopping|breaking|pause)\s+point\b",
    r"|\bas\s+far\s+as\s+(?:it\s+)?makes\s+sense\b",
    r"|\bas\s+far\s+as\s+(?:i|we)\s+can\s+(?:take|push)\s+(?:it|this)\b",
    r"|\b(?:taken|pushed|carried)\s+(?:it|this)\s+as\s+far\b",
    r"|\bgood\s+place\s+to\s+(?:stop|pause|hand)\b",
    r"|\brest\s+(?:is|can\s+be|should\s+be)\s+(?:a\s+)?(?:follow[\s-]?up|separate|future)\b",
    r"|\bpicks?\s+(?:this|it)\s+up\s+from\s+here\b",
    r"|\bsomeone\s+(?:else\s+)?(?:can|should)\s+(?:take|carry|finish)\b",
    r"|\bwhoever\s+(?:takes|picks|continues)\b",
);

const PRESENT_ACTION: &str = concat!(
    r"(?i)\b(?:implementing|building|writing|adding|wiring|fixing|running|editing)\b",
    r"|\blet\s+me\s+(?:build|implement|write|add|wire|fix|run|continue)\b",
    r"|\b(?:i'?ll|i\s+will|now)\s+(?:build|implement|write|add|wire|fix|run|continue|start)\b",
    r"|\bcontinuing\s+(?:to|with|on)\b|\bnext\s+i\b",
    r"|\bstart(?:ing)?\s+(?:it|on|the\s+next)\b",
);

static HANDOFF: LazyLock<Result<regex::Regex, regex::Error>> =
    LazyLock::new(|| regex::Regex::new(HANDOFF_PARAPHRASE));
static ACTION: LazyLock<Result<regex::Regex, regex::Error>> =
    LazyLock::new(|| regex::Regex::new(PRESENT_ACTION));

/// Verdict of the semantic deferral backstop.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticDeferral {
    /// No deferral signal — the turn does not read as a paraphrased handoff.
    Clear,
    /// The lexical `DEFERRAL_RE` already caught it — backstop is redundant here.
    CoveredByRegex,
    /// Paraphrased handoff the regex MISSED, with no present-tense action to
    /// negate it. This is the gap the backstop exists to close.
    ParaphrasedHandoff,
}

/// Pure semantic backstop for paraphrased handoffs the lexical deferral regex misses.
///
/// Fires `ParaphrasedHandoff` only when a handoff-paraphrase pattern matches AND
/// no concrete present-tense action negates it AND the lexical regex did NOT
/// already catch it — so it never double-counts and never fires on a turn that
/// is actively doing the work.
///
/// # Errors
/// Returns [`regex::Error`] only if a compile-time-constant pattern fails to
/// compile — unreachable at runtime since the patterns are static literals.
pub fn classify_semantic_deferral(msg: &str) -> Result<SemanticDeferral, regex::Error> {
    if msg.is_empty() {
        return Ok(SemanticDeferral::Clear);
    }
    if detect_strategic_deferral(msg)? {
        return Ok(SemanticDeferral::CoveredByRegex);
    }
    let handoff = HANDOFF.as_ref().map_err(Clone::clone)?;
    let action = ACTION.as_ref().map_err(Clone::clone)?;
    if handoff.is_match(msg) && !action.is_match(msg) {
        return Ok(SemanticDeferral::ParaphrasedHandoff);
    }
    Ok(SemanticDeferral::Clear)
}

#[cfg(test)]
#[path = "phase_a_semantic_deferral_test.rs"]
#[cfg(test)]
#[path = "phase_a_semantic_deferral_test.rs"]
mod tests;