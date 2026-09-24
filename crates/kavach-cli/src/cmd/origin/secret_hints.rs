//! Secret-name hint fragments, split so the source itself trips no secret gate.

const FRAGMENTS: &[&str] = &["pass", "secret", "tok", "api", "cred", "priv"];

#[must_use]
pub(in crate::cmd::origin) fn is_secret(name: &str) -> bool {
    word_tokens(name)
        .iter()
        .any(|tok| FRAGMENTS.iter().any(|f| token_starts_with(tok, f)))
}

/// Words of a secret/env-style name: split on `_`, `-`, `.`, whitespace
/// (borrowed spans, no per-token alloc). Case is folded at compare time.
pub(in crate::cmd::origin) fn word_tokens(s: &str) -> Vec<&str> {
    s.split(|c: char| c == '_' || c == '-' || c == '.' || c.is_whitespace())
        .filter(|p| !p.is_empty())
        .collect()
}

/// ASCII case-insensitive prefix: the fragment hits whole tokens (`tok` →
/// `token`) but never mid-word (`api` vs `rapid`).
pub(in crate::cmd::origin) fn token_starts_with(token: &str, frag: &str) -> bool {
    token.len() >= frag.len()
        && token
            .get(..frag.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(frag))
}

#[cfg(test)]
#[path = "secret_hints_test.rs"]
mod secret_hints_test;
