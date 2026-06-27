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

/// Disobedience-detector vocabulary AS DATA: floor + additive graph overlay.
///
/// Mirrors [`crate::stop_vocab::DoneGamingVocab`]: the compiled `const` lists are the
/// `Default` floor; a project's `gate.disobedience_vocab` DB row ADDS phrases on top
/// (research-refreshable, no rebuild). `#[serde(default)]` fills each list the row
/// omits from the floor, so a partial/malformed override degrades to the full floor —
/// the detector is never weaker than baseline (fail-closed). The graph ADDS, never
/// replaces: the floor markers + obey-proofs always apply. SOURCE: decision.w5 +
/// decision.userdirective.obey-not-argue.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct DisobedienceVocab {
    /// Argue-an-imperative-away phrases (lower-cased substrings).
    pub dismissal: Vec<String>,
    /// Markers proving an imperative fired in the same message.
    pub imperative_marker: Vec<String>,
    /// Obey-proof tokens whose presence clears the guard.
    pub obeyed: Vec<String>,
}

impl Default for DisobedienceVocab {
    fn default() -> Self {
        Self {
            dismissal: DISMISSAL.iter().map(|s| (*s).to_owned()).collect(),
            imperative_marker: IMPERATIVE_MARKER.iter().map(|s| (*s).to_owned()).collect(),
            obeyed: OBEYED.iter().map(|s| (*s).to_owned()).collect(),
        }
    }
}

/// `Some(reason)` when the message dismisses a fired imperative WITHOUT obey-proof.
///
/// Floor-default wrapper over [`detect_disobedience_with`] — the compiled vocabulary.
/// Fires on a dismissal phrase + an imperative marker + no obey-proof token.
#[must_use]
pub fn detect_disobedience(message: &str) -> Option<String> {
    detect_disobedience_with(&DisobedienceVocab::default(), message)
}

/// As [`detect_disobedience`], but against a resolved [`DisobedienceVocab`] (floor +
/// graph overlay). The match logic is identical; only the phrase source differs.
#[must_use]
pub fn detect_disobedience_with(vocab: &DisobedienceVocab, message: &str) -> Option<String> {
    let m = message.to_lowercase();
    let dismissed = vocab.dismissal.iter().find(|p| m.contains(p.as_str()))?;
    if !vocab
        .imperative_marker
        .iter()
        .any(|k| m.contains(k.as_str()))
    {
        return None;
    }
    if vocab.obeyed.iter().any(|p| m.contains(p.as_str())) {
        return None;
    }
    Some(format!(
        "dismissed a fired imperative in prose (\"{dismissed}\") with no obey-proof \
         (no URL / Loopholes closed: file:line / [RCA])"
    ))
}

#[cfg(test)]
#[path = "disobedience_guard_test.rs"]
#[cfg(test)]
mod tests;