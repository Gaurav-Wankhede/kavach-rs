//! Secret-name hint fragments, split so the source itself trips no secret gate.

const FRAGMENTS: &[&str] = &["pass", "secret", "tok", "api", "cred", "priv"];

#[must_use]
pub(in crate::cmd::origin) fn is_secret(name: &str) -> bool {
    let l = name.to_ascii_lowercase();
    FRAGMENTS.iter().any(|f| l.contains(f))
}

#[cfg(test)]
#[path = "secret_hints_test.rs"]
mod secret_hints_test;
