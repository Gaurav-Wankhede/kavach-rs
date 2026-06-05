use super::signal::Signal;
use std::sync::LazyLock;

const SYCOPHANCY_POS: &str = concat!(
    r"(?i)\b(?:great|excellent|amazing|impressive|wonderful|brilliant)\s+(?:work|job|effort|thinking|question)\b",
    r"|\byou're\s+(?:so\s+)?(?:smart|clever|talented|capable|brilliant)\b",
    r"|\b(?:love|adore|appreciate)\s+your\s+(?:work|style|approach)\b",
    r"|\b(?:you|your\s+[\w\s]{1,20}?)\s+(?:is|are)\s+(?:fantastic|awesome|stellar)\b",
    r"|\b(?:that'?s\s+a\s+)?great\s+point\b",
);

const SYCOPHANCY_NEG: &str = r"(?i)\bdetector\b|\bgate\b";

// An optional adverb may sit between the modal and the verb ("can't DIRECTLY
// edit"), so the verb slot is reached past a `\w+ly` adverb. The verb set
// covers file/tool actions the model wrongly disclaims — edit/modify/create/
// delete join the original access/read/write/execute/run set. Generalised
// (adverb-tolerant) rather than a literal-phrase list so paraphrases are caught.
const FALSE_INABILITY_POS: &str = concat!(
    r"(?i)\bi\s+(?:can't|cannot|don'?t\s+(?:have|know\s+how))\s+(?:\w+ly\s+)?",
    r"(?:access|read|write|edit|modify|create|delete|execute|run|run\s+code)\b",
    r"|\b(?:I'm\s+unable|I\s+lack\s+the\s+(?:ability|capability))\s+to\b",
    r"|\bI\s+(?:don't\s+)?have\s+access\s+to\b",
    r"|\bI\s+can't\s+(?:verify|check|confirm|validate)\s+(?:that\s+)?directly\b",
    // Passive tool-block narration: "read and write tools are blocked".
    r"|\b\w+(?:\s+and\s+\w+)?\s+tools?\s+(?:is|are)\s+blocked\b",
    // Delegation framing: "only way is for you to run ...".
    r"|\bonly\s+way\s+is\s+for\s+you\s+to\b",
);

const FALSE_INABILITY_NEG: &str = r"(?i)\byet\b|\bwithout\s+(?:your|explicit)\s+help\b";

const INCOMPLETE_WORK_POS: &str = concat!(
    r"(?i)\b(?:I'?ll\s+)?leave\s+(?:that\s+)?(?:to|for)\s+you\b",
    r"|\b(?:the\s+rest|remaining\s+work|next\s+step)\b[\w\W]{0,40}?\bis\s+(?:yours|up\s+to\s+you)\b",
    r"|\byou\s+can\s+(?:now|then)\s+(?:run|apply|implement|do|execute)\b",
    r"|\bready\s+when\s+you\s+(?:are|want\s+to\s+proceed)\b",
    // Discovery announced ("found/detected ..."): a positive trigger; the
    // negation guard below suppresses it when a completion marker co-occurs.
    r"|\b(?:found|discovered|detected|identified)\b",
    // Stated intent to act, not yet acted ("need to remove ...").
    r"|\bneed\s+to\s+\w+",
    // In-progress promise with no completion ("Fixing now").
    r"|\b\w+ing\s+now\b",
);

// Negation guard: a completion marker (past-tense resolution, or "0 errors")
// means the discovered work WAS finished — so the discovery trigger must not
// fire. `fixed`/`removed`/`resolved` are past-tense ONLY; the bare in-progress
// forms ("Fixing", "remove") stay outside the guard so they still fire.
const INCOMPLETE_WORK_NEG: &str = concat!(
    r"(?i)\b(?:as\s+context\s+permits|along\s+the\s+way)\b",
    r"|\b(?:fixed|removed|resolved|deleted|completed|done)\b",
    r"|\b0\s+errors?\b",
);

static SYCOPHANCY: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(SYCOPHANCY_POS)),
    negation: LazyLock::new(|| regex::Regex::new(SYCOPHANCY_NEG)),
};

static FALSE_INABILITY: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(FALSE_INABILITY_POS)),
    negation: LazyLock::new(|| regex::Regex::new(FALSE_INABILITY_NEG)),
};

static INCOMPLETE_WORK: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(INCOMPLETE_WORK_POS)),
    negation: LazyLock::new(|| regex::Regex::new(INCOMPLETE_WORK_NEG)),
};

/// # Errors
/// Returns [`regex::Error`] only if a compile-time-constant pattern fails to
/// compile — unreachable at runtime since the patterns are static literals.
pub fn detect_sycophancy(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    SYCOPHANCY.fires(msg)
}

/// # Errors
/// Returns [`regex::Error`] only if a compile-time-constant pattern fails to
/// compile — unreachable at runtime since the patterns are static literals.
pub fn detect_false_inability(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    FALSE_INABILITY.fires(msg)
}

/// # Errors
/// Returns [`regex::Error`] only if a compile-time-constant pattern fails to
/// compile — unreachable at runtime since the patterns are static literals.
pub fn detect_incomplete_work(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    INCOMPLETE_WORK.fires(msg)
}
