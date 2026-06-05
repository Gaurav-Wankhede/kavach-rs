//! Additional SOLID checks that don't fit the main categories.

use crate::solid_guard::{SolidLetter, SolidSeverity, SolidViolation};

fn push(
    v: &mut Vec<SolidViolation>,
    severity: SolidSeverity,
    letter: SolidLetter,
    pattern: &'static str,
    fix: &'static str,
) {
    v.push(SolidViolation {
        severity,
        letter,
        pattern,
        fix,
        line: 0,
    });
}

pub(super) fn check_handler_raw_request(
    p: &[regex::Regex],
    content: &str,
    v: &mut Vec<SolidViolation>,
) {
    if p.get(20).is_some_and(|re| re.is_match(content)) {
        push(
            v,
            SolidSeverity::P2Warning,
            SolidLetter::L,
            "axum-handler-raw-request",
            "Handler takes raw Request<...> instead of typed extractors (State/Path/Query/Json). Bypasses Axum's typed contracts.",
        );
    }
}
