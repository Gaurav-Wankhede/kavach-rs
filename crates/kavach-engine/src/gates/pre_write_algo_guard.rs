//! Algorithm Hunter pre-write guard.
//!
//! ALGO: aho-corasick
//! `PROBLEM_CLASS`: `string_match`
//! REJECTED: [{"name":"naive-scan","reason":"O(n*k) per keyword, degrades with trigger list growth"}]
//! TIME: O(n+m) | SPACE: O(m*sigma)
//! YEAR: 1975 | SEARCHED: 2026-04
//! TRADEOFF: DFA construction cost upfront; for small static keyword sets, linear scan is comparable
//! BENCHMARK: <https://crates.io/crates/aho-corasick>
//!
//! Blocks writes to Rust files that introduce non-trivial algorithmic logic
//! without prior `/arch` invocation this turn, OR auto-injects prior algorithm
//! decisions from kavach-db when available.
//!
//! Three outcomes:
//! - `Allow` — no trigger, hunter already invoked, or `// ALGO:` comment present
//! - `AutoInject(ctx)` — trigger found, but prior DB decision exists; advisory
//! - `Block(msg)` — trigger found, no prior decision, hunter not invoked
mod check;
mod decision;
mod outcome;
mod strip;
mod triggers;

#[cfg(test)]
mod tests;

pub(crate) use check::check;
pub(crate) use outcome::AlgoGuardOutcome;
