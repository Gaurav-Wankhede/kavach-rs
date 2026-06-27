//! Env-key signal: does the candidate name contain a role env-key hint fragment.

#[must_use]
pub(in crate::cmd::origin) fn score(hints: &[String], name: &str) -> f32 {
    if hints.is_empty() {
        return 0.0;
    }
    let l = name.to_ascii_lowercase();
    let hit = hints.iter().any(|h| {
        let h = h.trim().to_ascii_lowercase();
        !h.is_empty() && l.contains(&h)
    });
    f32::from(u8::from(hit))
}

#[cfg(test)]
#[path = "env_key_test.rs"]
mod env_key_test;
