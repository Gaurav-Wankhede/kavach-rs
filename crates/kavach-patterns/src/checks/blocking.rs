use crate::config::load;
use kavach_config::blocking_util::blocked_pattern_match;

#[must_use]
pub fn is_blocked(cmd: &str) -> bool {
    let g = load();
    let cl = cmd
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    g.as_ref().is_some_and(|c| {
        c.blocked.iter().any(|p| {
            let pl = p.to_lowercase();
            blocked_pattern_match(&cl, &pl)
        })
    })
}
