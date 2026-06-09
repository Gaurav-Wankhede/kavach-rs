//! Whole-content `GoF` scanners for the design-patterns guard. Split out of
//! `design_patterns_guard.rs` to keep each module under the 100-LOC bar.
//!
//! Each returns the 1-based line of the first match, or `None`.
//! SOURCE: <https://refactoring.guru/design-patterns/rust>

use regex::Regex;
use std::sync::LazyLock;

// GoF State: `mem::take(&mut self.<field>)` needs `Default`, absent on `Box<dyn _>`
// (E0277). Fix: `Option<Box<dyn _>>` + `Option::take()`.
static RE_STATE_TAKE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"mem::take\s*\(\s*&mut\s+self\.\w+\s*\)").ok());

// GoF Flyweight: `&mut self` accessor returning `&T` aliases the &mut borrow (E0499).
static RE_FLYWEIGHT_MUT_REF: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"fn\s+\w+\s*\(\s*&mut\s+self\b[^)]*\)\s*->\s*&\s*\w").ok());

static RE_NEW_CONSTRUCTOR: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"fn\s+new\s*\(([^)]*)\)").ok());

fn line_at(content: &str, byte: usize) -> usize {
    content
        .get(..byte)
        .map_or(0, |s| s.matches('\n').count())
        .saturating_add(1)
}

pub(crate) fn state_take_on_boxdyn(content: &str) -> Option<usize> {
    let re = RE_STATE_TAKE.as_ref()?;
    re.find(content).map(|m| line_at(content, m.start()))
}

pub(crate) fn flyweight_mut_ref(content: &str) -> Option<usize> {
    let re = RE_FLYWEIGHT_MUT_REF.as_ref()?;
    re.find(content).map(|m| line_at(content, m.start()))
}

pub(crate) fn many_arg_constructor(content: &str) -> Option<usize> {
    let re = RE_NEW_CONSTRUCTOR.as_ref()?;
    for m in re.captures_iter(content) {
        let Some(args_match) = m.get(1) else { continue };
        if args_match.as_str().matches(',').count() >= 4 {
            let Some(full) = m.get(0) else { continue };
            return Some(line_at(content, full.start()));
        }
    }
    None
}
