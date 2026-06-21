//! Detects "willful disobedience".
//!
//! The model dismisses a fired imperative in prose instead of executing its
//! mandated tool call. SOURCE: arxiv 2603.23806 (Willful Disobedience detector)
//! + `decision.engine.disobedience-guard`.

/// Dismissal phrases the model uses to argue an imperative away rather than obey.
const DISMISSAL: &[&str] = &[
    "n/a here",
    "n/a —",
    "n/a -",
    "n/a:",
    "doesn't apply",
    "does not apply",
    "not applicable",
    "no need to",
    "comment-only so",
    "comment-only edit",
    "i'll just verify myself",
    "let me just verify myself",
    "the agent over-claimed",
    "rather than argue",
    "no reason to",
    "can be ignored",
    "safe to skip",
    "skip the lens",
    "lens: n/a",
];

/// Imperative markers whose presence in the SAME message means an imperative fired
/// and is being responded to. Pairing a marker with a dismissal = argue-not-obey.
const IMPERATIVE_MARKER: &[&str] = &[
    "loophole",
    "[research",
    "research-first",
    "research first",
    "agent-spawn",
    "[advisory",
    "attention_dilution",
    "[invoke_agent",
    "internet-first",
    "websearch",
];

/// Proof tokens that show the mandated action was actually taken this turn — their
/// presence clears the guard (obeyed, not argued).
const OBEYED: &[&str] = &[
    "loopholes closed:",
    "sources:",
    "http://",
    "https://",
    "[rca]",
];

/// `Some(reason)` when the message dismisses a fired imperative WITHOUT proof it was
/// obeyed: a dismissal phrase + an imperative marker + no obey-proof token. `None`
/// otherwise (no marker, no dismissal, or proof present).
#[must_use]
pub fn detect_disobedience(message: &str) -> Option<String> {
    let m = message.to_lowercase();
    let dismissed = DISMISSAL.iter().find(|p| m.contains(**p))?;
    if !IMPERATIVE_MARKER.iter().any(|k| m.contains(k)) {
        return None;
    }
    if OBEYED.iter().any(|p| m.contains(p)) {
        return None;
    }
    Some(format!(
        "dismissed a fired imperative in prose (\"{dismissed}\") with no obey-proof \
         (no URL / Loopholes closed: file:line / [RCA])"
    ))
}

#[cfg(test)]
#[path = "disobedience_guard_test.rs"]
mod tests;
