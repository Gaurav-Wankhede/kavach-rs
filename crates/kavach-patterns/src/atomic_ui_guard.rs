//! Atomic UI Production Gate — Framework-Agnostic
//!
//! Enforces Brad Frost's Atomic Design (5 chapters) across React, Vue, Svelte,
//! Solid, Astro, Dioxus, Yew, Leptos. Aligned with 2026 Contract-Driven Design
//! evolution — atoms/molecules/organisms structure becomes an enforceable contract.
//!
//! HIERARCHY: Pages → Templates → Organisms → Molecules → Atoms → (Tokens)
//!
//! IMPORT CONTRACT:
//!   Atoms     ← tokens, std primitives only
//!   Molecules ← atoms, tokens
//!   Organisms ← molecules, atoms, tokens
//!   Templates ← organisms, molecules, atoms, tokens
//!   Pages     ← anything
//!
//! SOURCES (verified 2026-05):
//! - <https://atomicdesign.bradfrost.com/table-of-contents>/
//! - <https://atomicdesign.bradfrost.com/chapter-1>/
//! - <https://atomicdesign.bradfrost.com/chapter-2>/
//! - <https://atomicdesign.bradfrost.com/chapter-3>/
//! - <https://atomicdesign.bradfrost.com/chapter-4>/
//! - <https://atomicdesign.bradfrost.com/chapter-5>/
//! - <https://designtokenscourse.com>/
//! - <https://aianddesign.systems/#content>
//! - <https://atomicdesigncourse.com>/
//! - <https://medium.com/@iz.iuqo/atomic-design-reached-its-peak-contract-driven-design-is-what-comes-next-9174a9a89aea>

mod detectors;
mod types;
mod util;

#[cfg(test)]
#[path = "atomic_ui_guard/tests.rs"]
mod tests;

pub use types::{AtomicSeverity, AtomicViolation};

use types::Level;
use util::{classify_path, is_ui_file};

#[must_use]
pub fn detect(file_path: &str, content: &str) -> Vec<AtomicViolation> {
    if !is_ui_file(file_path, content) {
        return vec![];
    }
    if crate::file_types::is_test_file(file_path) {
        return vec![];
    }

    let level = classify_path(file_path);
    detectors::dispatch(file_path, content, level)
}
