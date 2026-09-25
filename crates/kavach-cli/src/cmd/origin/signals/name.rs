//! Name signal: exact/alias/normalized-token jaccard — 0 for an arbitrary name, never blocking.
//!
//! Tokens are borrowed spans (no per-token `String`): the split predicate is
//! unchanged, and set comparison is case-insensitive on the spans.

#[must_use]
pub(in crate::cmd::origin) fn score(aliases: &[String], name: &str) -> f32 {
    if aliases.is_empty() {
        return 0.0;
    }
    let cand = tokens(name);
    aliases
        .iter()
        .map(|a| {
            if a.eq_ignore_ascii_case(name) {
                1.0
            } else {
                jaccard(&cand, &tokens(a))
            }
        })
        .fold(0.0_f32, f32::max)
}

/// Word spans over camel/snake/kebab/dot boundaries. Allocation-free per
/// token: one `Vec` of borrowed slices, deduped case-insensitively.
fn tokens(s: &str) -> Vec<&str> {
    let mut out: Vec<&str> = Vec::new();
    for tok in s
        .split(|c: char| c == '_' || c == '-' || c == '.' || c.is_whitespace() || c.is_uppercase())
        .filter(|p| !p.is_empty())
    {
        if !out.iter().any(|t| t.eq_ignore_ascii_case(tok)) {
            out.push(tok);
        }
    }
    out
}

// SOURCE: rust-lang.github.io/rust-clippy/master/index.html#float_arithmetic
#[expect(
    clippy::float_arithmetic,
    reason = "Jaccard similarity ratio, not money"
)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "inter <= min(a,b) so a.len()+b.len()-inter never underflows"
)]
#[expect(
    clippy::cast_precision_loss,
    reason = "set sizes are small alias counts, well within f32 mantissa"
)]
fn jaccard(a: &[&str], b: &[&str]) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let inter = a
        .iter()
        .filter(|x| b.iter().any(|y| x.eq_ignore_ascii_case(y)))
        .count();
    let union = a.len() + b.len() - inter;
    if union == 0 {
        0.0
    } else {
        inter as f32 / union as f32
    }
}

#[cfg(test)]
#[path = "name_test.rs"]
mod name_test;
