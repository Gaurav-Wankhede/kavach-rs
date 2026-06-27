//! Stop-gate done-detection vocabulary AS DATA.
//!
//! The completion-narration and operator-handback marker lists the done-gaming
//! gate matches against, sourced from the kavach DB (`gate.done_gaming_vocab` row)
//! at runtime rather than frozen as compiled `const` arrays. The `Default` impl IS
//! the compiled floor: when the
//! DB is unreachable / the row is absent / the blob is malformed, the gate falls
//! back to these exact phrases, so it is never weaker than its hardcoded baseline.
//!
//! Mirrors `reward::oracle::OracleConfig` (the `#[serde(default)]` config-as-data
//! pattern): each list a DB row omits is filled from the default, so a partial
//! override is honored and a malformed blob degrades to the full floor.

/// Done-by-redefinition + narration/sign-off phrases (compiled floor).
///
/// Lower-cased substring match. Each is a completion-claim the model uses to END a
/// turn without doing the work, NOT generic prose. The `Default` for the DB list.
pub const DEFAULT_GAMING_PHRASES: &[&str] = &[
    // done-by-redefinition
    "vacuously complete",
    "vacuous",
    "await features",
    "awaiting features",
    "shipped for live types",
    "documentation pass",
    "doc pass",
    "safe-by-construction",
    "nothing further is runnable",
    "must not run",
    // narration / sign-off
    "the new status block",
    "status block:",
    "that completes",
    "one-line summary",
    "single source of truth for the migration",
];

/// Operator-handback / surrender phrases (compiled floor).
///
/// The abolished "push it to the owner" pattern. Fires unconditionally of the proof
/// NEG-arm. Lower-cased substring match. `Default` for the DB-sourced handback list.
pub const DEFAULT_HANDBACK_PHRASES: &[&str] = &[
    "owner — run",
    "owner - run",
    "owner must free",
    "owner must run",
    "owner-authorization anchor",
    "run in your terminal",
    "no agent action can",
    "only an external",
    "holding for",
    "holding until",
    "i'm holding",
    "i am holding",
];

/// The Stop gate's done-detection vocabulary, resolved per project from the DB.
///
/// `#[serde(default)]`: a row omitting either list keeps that list's compiled floor,
/// so a partial override never blanks a dimension. `#[non_exhaustive]`: new vocab
/// axes can be added without breaking downstream matches.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct DoneGamingVocab {
    /// Completion-narration / done-by-redefinition phrases (lower-cased substrings).
    pub gaming_phrases: Vec<String>,
    /// Operator-handback / surrender phrases (lower-cased substrings).
    pub handback_phrases: Vec<String>,
}

impl Default for DoneGamingVocab {
    fn default() -> Self {
        Self {
            gaming_phrases: DEFAULT_GAMING_PHRASES
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
            handback_phrases: DEFAULT_HANDBACK_PHRASES
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
        }
    }
}

impl DoneGamingVocab {
    /// `true` when `lc` (a lower-cased message) contains any gaming phrase.
    #[must_use]
    pub fn has_gaming_phrase(&self, lc: &str) -> bool {
        self.gaming_phrases.iter().any(|p| lc.contains(p.as_str()))
    }

    /// `true` when `lc` (a lower-cased message) contains any handback phrase.
    #[must_use]
    pub fn has_handback_phrase(&self, lc: &str) -> bool {
        self.handback_phrases
            .iter()
            .any(|p| lc.contains(p.as_str()))
    }
}

#[cfg(test)]
#[path = "stop_vocab_test.rs"]
#[cfg(test)]
mod tests;