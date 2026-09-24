//! Env-key signal: does the candidate name contain a role env-key hint fragment.

use super::super::secret_hints::{token_starts_with, word_tokens};

#[must_use]
pub(in crate::cmd::origin) fn score(hints: &[String], name: &str) -> f32 {
    if hints.is_empty() {
        return 0.0;
    }
    // Token-prefix matching on shared word spans (no per-hint lowercased
    // copies): a hint hits when it prefixes a whole token, never mid-word.
    let tokens = word_tokens(name);
    let hit = hints.iter().any(|h| {
        let h = h.trim();
        !h.is_empty() && tokens.iter().any(|tok| token_starts_with(tok, h))
    });
    f32::from(u8::from(hit))
}

#[cfg(test)]
#[path = "env_key_test.rs"]
mod env_key_test;
