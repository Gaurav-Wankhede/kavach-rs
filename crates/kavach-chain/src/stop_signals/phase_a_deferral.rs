use super::signal::Signal;
use std::sync::LazyLock;

const STRATEGIC_DEFERRAL_POS: &str = concat!(
    r"(?i)\bpost[\s-]?(?:launch|release|mvp|ship)\b",
    r"|\bafter\s+(?:launch|release|shipping|go[\s-]?live)\b",
    r"|\b(?:after|once)\s+we(?:'ve|\s+have)?\s+ship(?:ped|ping)?\b",
    r"|\bphase\s*(?:2|3|4|ii|iii|iv|two|three|four)\b",
    r"|\bv[2-9]\s+(?:release|feature|candidate|milestone)\b",
    r"|\b(?:should\s+be|make\s+it|move\s+to)\s+v[2-9]\b",
    r"|\bnext\s+(?:iteration|version|release|sprint|cycle|milestone)\b",
    r"|\bfuture\s+(?:release|iteration|version|enhancement|milestone)\b",
    r"|\bverdict:\s*(?:not\s+(?:now|yet)|no)\b",
    r"|\bnot\s+(?:now|yet|appropriate\s+(?:now|yet)|the\s+right\s+time)\b[.,;\u{2014}\u{2013}]",
    r"|\bpremature\s+(?:optimi[sz]ation|to\s+(?:implement|add|build|optimi[sz]e))\b",
    r"|\bnot\s+(?:critical|needed|required)\s+(?:now|yet)\b",
    r"|\bcan\s+wait\s+(?:until|for)\b",
    r"|\b(?:later|future)\s+optimi[sz]ation\b",
    r"|\bnoted\s+for\s+(?:future|later)\b",
    r"|\b(?:out\s+of|outside)\s+scope\b",
    r"|\bdeferred\s+to\b|\bbacklog(?:\s+item|ged)?\b|\badd\s+to\s+backlog\b",
    r"|\b(?:implementation|recommended)\s+order\s*[:(]",
    r"|\b(?:pre|post)[\s-]?launch\s*[:(]",
    r"|\b(?:now|next|later|post[\s-]?launch|pre[\s-]?launch)\s*\(\s*",
    r"(?:pre[\s-]?launch|post[\s-]?launch|[0-9]+\s*days?|critical|important)\b",
    r"|\bnot\s+implemented\s*[\u{2014}\u{2013}\n]",
    r"|\b(?:next|future|later|another|subsequent|fresh|new|dedicated|separate|clean)\b\W*(?:\w+\W+){0,2}?\b(?:session|turn|chat|window|conversation|run|invocation|context)\b",
    r"|\b(?:next|future|later|another|subsequent)\s+(?:time|pass|round|go)\b",
    r"|\b(?:defer(?:red|ring|s)?|postpon(?:e|ed|ing|es)|shelv(?:e|ed|ing|es)",
    r"|tabl(?:e|ed|ing)\s+(?:this|that|it)|park(?:ed|ing)?\s+(?:this|that|it)",
    r"|revisit(?:ed|ing)?\s+(?:this|that|it|later))\b",
    r"|\b(?:sav(?:e|ed|ing)|leav(?:e|ing)|left|push(?:ed|ing)?)\b\W*(?:\w+\W+){0,3}?\bfor\s+later\b",
    r"|\bstretch\s+goal\b|\bnice[-\s]to[-\s]have\b|\bif\s+(?:we\s+have\s+)?time\b|\btime\s+permitting\b",
    r"|\b(?:lower|low|de)[-\s]?prioriti[sz]e?d?\b",
    r"|\bremaining\s+work\b|\bwork\s+(?:still\s+)?(?:to\s+do|left|remaining)\b",
);

const DOING_IT_NOW: &str = concat!(
    r"(?i)\b(?:implementing|building|starting|doing\s+it|writing\s+it)\s+now\b",
    r"|\bbuilding\s+both\b|\blet\s+me\s+(?:build|implement|start|write)\b",
    r"|\bwill\s+(?:implement|build|do\s+it|write\s+it)\s+now\b",
    r"|\b(?:implementing|building|doing|adding)\s+(?:it\s+|this\s+|them\s+)?(?:anyway|regardless|now)\b",
    r"|\b(?:your\s+)?next\s+\w+\s+will\s+(?:have|use|inherit|see|get)\b",
);

static STRATEGIC_DEFERRAL: Signal = Signal {
    positive: LazyLock::new(|| regex::Regex::new(STRATEGIC_DEFERRAL_POS)),
    negation: LazyLock::new(|| regex::Regex::new(DOING_IT_NOW)),
};

/// # Errors
/// Returns [`regex::Error`] only if a compile-time-constant pattern fails to
/// compile — unreachable at runtime since the patterns are static literals.
pub fn detect_strategic_deferral(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    STRATEGIC_DEFERRAL.fires(msg)
}
