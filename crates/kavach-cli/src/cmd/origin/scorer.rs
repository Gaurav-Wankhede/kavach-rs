//! Multi-signal scorer: resolve a config var by ROLE, not NAME.

use super::role_query::{Candidate, RoleQuery};
use super::signals;

pub(super) const THRESHOLD: f32 = 0.6;

#[derive(Debug, Clone)]
pub(super) struct Scored {
    pub cand: Candidate,
    pub score: f32,
}

#[must_use]
pub(super) fn rank(q: &RoleQuery, cands: Vec<Candidate>) -> Vec<Scored> {
    let mut out: Vec<Scored> = cands
        .into_iter()
        .map(|c| {
            let score = score_one(q, &c);
            Scored { cand: c, score }
        })
        .filter(|s| s.score >= THRESHOLD)
        .collect();
    out.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.cand.kind.cmp(&b.cand.kind))
            .then_with(|| (a.cand.file.clone(), a.cand.line).cmp(&(b.cand.file.clone(), b.cand.line)))
    });
    out
}

// SOURCE: rust-lang.github.io/rust-clippy/master/index.html#float_arithmetic
#[expect(clippy::float_arithmetic, reason = "relevance scoring weights, not money or safety-critical")]
fn score_one(q: &RoleQuery, c: &Candidate) -> f32 {
    let val = signals::value::score(q.value_regex.as_deref(), c.value.as_deref());
    let cons = signals::consumption::score(&q.consumed_by, c);
    let env = signals::env_key::score(&q.env_key_hints, &c.name);
    let name = signals::name::score(&q.name_aliases, &c.name);
    let mut s = 0.35 * val + 0.35 * cons + 0.2 * env + 0.1 * name;
    if c.kind.is_centralized() {
        s += 0.05;
    }
    let precise_hit = val >= 1.0 || cons >= 1.0;
    if precise_hit {
        s = s.max(THRESHOLD + 0.05 * (val + cons));
    }
    s.min(1.0)
}

#[cfg(test)]
#[path = "scorer_test.rs"]
mod scorer_test;
