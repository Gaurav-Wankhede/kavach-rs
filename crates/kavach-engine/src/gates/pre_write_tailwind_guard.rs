//! Tailwind Plus advisory gate — injects a matching component reference on
//! frontend writes (.tsx/.jsx/.astro). Output: `[TAILWIND_PLUS_REF]` advisory
//! block — never a hard block.
mod advisory;
mod keywords;
mod matching;

#[cfg(test)]
mod tests;

pub(crate) use advisory::advisory;
