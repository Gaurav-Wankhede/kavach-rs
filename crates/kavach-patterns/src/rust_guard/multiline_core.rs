//! Multiline (whole-content) core detector arms (indices 20-58), split into two
//! leaves to honor the ≤100-LOC law: `structural` (20-44: empty-body, status,
//! encapsulation, abstraction, router, async, DSA) and `discard_race` (45-58:
//! magic literals, error-mapping, silent-discard, data races).
use crate::severity::Violation;
use regex::Regex;

mod discard_race;
mod structural;

/// Run both core sub-scans over the whole `content`.
pub(super) fn scan(r: &[Regex], content: &str, v: &mut Vec<Violation>) {
    structural::scan(r, content, v);
    discard_race::scan(r, content, v);
}
