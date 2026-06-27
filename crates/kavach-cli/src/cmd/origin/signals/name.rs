//! Name signal: exact/alias/normalized-token jaccard — 0 for an arbitrary name, never blocking.

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

fn tokens(s: &str) -> Vec<String> {
    let mut t: Vec<String> = s
        .split(|c: char| c == '_' || c == '-' || c == '.' || c.is_whitespace() || c.is_uppercase())
        .filter(|p| !p.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    if t.is_empty() {
        t.push(s.to_ascii_lowercase());
    }
    t.sort();
    t.dedup();
    t
}

fn jaccard(a: &[String], b: &[String]) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let inter = a.iter().filter(|x| b.contains(x)).count();
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
