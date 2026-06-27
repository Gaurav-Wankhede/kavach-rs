//! Tailwind Plus advisory gate — injects a matching component reference on
//! frontend writes (.tsx/.jsx/.astro). Output: `[TAILWIND_PLUS_REF]` advisory
//! block — never a hard block.
mod advisory;
mod keywords;
mod matching;
#[cfg(test)]
#[path = "pre_write_tailwind_guard_test.rs"]
mod tests;
pub(crate) use advisory::advisory;
