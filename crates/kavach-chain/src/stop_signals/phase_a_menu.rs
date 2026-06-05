use super::signal::{NEVER, Signal};
use std::sync::LazyLock;

const CONTINUATION_MENU_POS: &str = concat!(
    r"(?i)\bsay\s+[\x22\x27]?[\w\s./-]{1,40}?[\x22\x27]?\s+and\s+i'?ll\s+(?:proceed|continue|pick\s+up|resume)\b",
    r"|\b(?:continue|proceed|carry\s+on|keep\s+going)\b[\w\W]{0,80}?\bor\b[\w\W]{0,40}?\b(?:switch|redirect|pivot|jump\s+to|move\s+to|change\s+threads?)\b",
    r"|\b(?:switch|redirect|pivot)\b[\w\W]{0,80}?\bor\b[\w\W]{0,40}?\b(?:continue|proceed|carry\s+on)\b",
    r"|\b(?:want|would\s+you\s+like|do\s+you\s+want)\s+me\s+to\s+(?:continue|proceed|keep\s+going)\b[\w\W]{0,40}?\bor\b",
    r"|\blet\s+me\s+know\s+(?:which|if\s+you'?d\s+like\s+me\s+to\s+(?:continue|switch|proceed))\b",
    r"|\b(?:which\s+(?:thread|one|do\s+you\s+want)|your\s+call)\b[\w\W]{0,30}?\b(?:proceed|continue|next)\b",
    r"|\b(?:proceed|continue|next)\b[\w\W]{0,30}?\b(?:your\s+call|let\s+me\s+know|which\s+(?:thread|one))\b",
    r"|\bnext\s+(?:card|step|task|thing|up|one)\b[\w\W]{0,60}?\b(?:unless\s+you|want\s+me\s+to|let\s+me\s+know|if\s+you'?d\s+(?:like|prefer)|(?:or\s+)?(?:redirect|pivot|switch))\b",
    r"|\b(?:unless\s+you|want\s+me\s+to|let\s+me\s+know|if\s+you'?d\s+(?:like|prefer))\b[\w\W]{0,60}?\bnext\s+(?:card|step|task|thing|up|one)\b",
    r"|\bunless\s+you'?d\s+(?:like|want|prefer)\s+(?:me\s+)?to\s+(?:redirect|pivot|switch|change|pick)\b",
);

const CONTINUATION_MENU_NEG: &str = concat!(
    r"(?i)\byou\s+asked\s+(?:me\s+)?to\s+(?:choose|pick|decide)\b",
    r"|\bas\s+you\s+(?:requested|asked)\b|\bper\s+your\s+(?:request|instruction)\b",
    r"|\bgenuinely\s+ambiguous\b|\bchanges?\s+the\s+(?:outcome|result)\b",
    r"|\b(?:destructive|irreversible)\b[\w\W]{0,40}?\b(?:authoriz|permission|confirm)\b",
    r"|\b(?:missing|need|require)s?\s+(?:a\s+)?(?:credential|secret|token|api\s+key|password|access)\b",
    r"|\bdetector\b|\bgate\s+(?:catches|blocks|fires)\b|\bstop_detect\b|\bstop\s+gate\b",
);

const STRONG_SCOPE_ASK_POS: &str = concat!(
    r"(?i)(?:\byour\s+call\b|\bwhich\s+(?:one\s+)?do\s+you\s+want\b",
    r"|\bwhat\s+would\s+you\s+like\b|\bhow\s+would\s+you\s+like\b",
    r"|\bdo\s+you\s+want\s+me\s+to\b|\bshould\s+i\s+\w+",
    r"|\bwhich\s+(?:approach|option)\s+do\s+you)[\s\S]{0,200}\?",
);

static CONTINUATION_MENU: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(CONTINUATION_MENU_POS)),
    negation: LazyLock::new(|| regex::Regex::new(CONTINUATION_MENU_NEG)),
};

static STRONG_SCOPE_ASK: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(STRONG_SCOPE_ASK_POS)),
    negation: LazyLock::new(|| regex::Regex::new(NEVER)),
};

/// # Errors
/// Returns [`regex::Error`] only if a compile-time-constant pattern fails to
/// compile — unreachable at runtime since the patterns are static literals.
pub fn detect_continuation_menu(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    CONTINUATION_MENU.fires(msg)
}

/// # Errors
/// Returns [`regex::Error`] only if a compile-time-constant pattern fails to
/// compile — unreachable at runtime since the patterns are static literals.
pub fn detect_strong_scope_ask(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    STRONG_SCOPE_ASK.fires(msg)
}
