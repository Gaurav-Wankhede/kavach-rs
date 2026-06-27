//! Consumption signal: is the candidate's name fed into one of the role's consumer symbols.

use crate::cmd::origin::role_query::Candidate;

#[must_use]
pub(in crate::cmd::origin) fn score(consumed_by: &[String], c: &Candidate) -> f32 {
    if consumed_by.is_empty() || matches!(c.kind, Kind::Function | Kind::Type) {
        return 0.0;
    }
    let hay = c.value.as_deref().unwrap_or("");
    let hit = consumed_by.iter().any(|sym| {
        let s = sym.trim();
        !s.is_empty() && (hay.contains(s) || c.name.contains(s))
    });
    f32::from(u8::from(hit))
}

#[cfg(test)]
#[path = "consumption_test.rs"]
mod consumption_test;
