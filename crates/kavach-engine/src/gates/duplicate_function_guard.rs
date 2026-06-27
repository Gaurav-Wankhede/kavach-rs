//! Duplicate-function guard.
//!
//! Detects copy-paste functions by comparing token-shingle Jaccard similarity
//! between a candidate function and existing functions in the same crate.
//!
//! ALGO `TokenShingle` + Jaccard; `PROBLEM_CLASS` `near_duplicate_detection`.
//!
//! Rejected `MinHash+LSH` (external crate, overkill for <1k functions) and AST
//! diff (`syn` dep heavy, AST-equivalence != duplication intent).
//!
//! TIME `O(n*k)` signature build, `O(n^2)` pairwise compare per crate; SPACE
//! `O(n*k)` signatures. `O(n^2)` is acceptable for <1k functions per crate;
//! switch to `MinHash+LSH` at scale.
//!
//! YEAR 1997 Broder (shingles) + 2014 Manning IR; SEARCHED 2026-05.
//! SOURCE <https://mbrenndoerfer.com/writing/minhash-algorithm-jaccard-similarity-lsh-deduplication>,
//! <https://blog.nelhage.com/post/fuzzy-dedup/> (Jaccard threshold guidance).
mod decision;
mod shingle;
#[cfg(test)]
#[path = "duplicate_function_guard_test.rs"]
#[cfg(test)]
#[path = "duplicate_function_guard_test.rs"]
mod tests;