//! TypeScript Production Guard — pre-write gate for .ts/.tsx/.jsx/.astro files.
//! P0 violations (as any, hardcoded URLs, mock data, XSS) = HARD BLOCK.
//! P1 violations (console.log, direct DOM) = advisory warning.
//! Component monoliths (>100 lines, 2+ exported components) = block.
mod block;
mod component;

#[cfg(test)]
mod tests;

pub(crate) use block::{check, format_advisory};
pub(crate) use component::check_component_oversized;
