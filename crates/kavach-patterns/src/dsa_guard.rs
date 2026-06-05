// split: Single-module DSA gate. Test fixtures intentionally embed anti-pattern Rust source.
//
// ALGO: Aho-Corasick (via regex crate) + LazyLock-cached pattern set
// PROBLEM_CLASS: multi-pattern static text scan over Rust source
// REJECTED: [
//   {"name":"syn AST walk","reason":"3-5x slower for write-time gate; needs full parse"},
//   {"name":"tree-sitter","reason":"adds 2MB binary, overkill for ~16 regex matches"},
//   {"name":"hand-rolled scanner","reason":"reinvents regex DFA poorly"}
// ]
// TIME: O(n) per file (single-pass NFA via regex crate) | SPACE: O(patterns)
// YEAR: 2026 | SEARCHED: 2026-05
// TRADEOFF: regex misses semantic context; we accept false-positives at P1/P2 advisory tier
// BENCHMARK: https://docs.rs/regex/latest/regex/#performance
//! DSA Gate — Data Structures & Algorithms for Rust Backends
//!
//! Detects accidental O(n) / O(n^2) / quadratic-allocation patterns that scale poorly under load.
//!
//! SOURCES (verified 2026-05):
//! - <https://doc.rust-lang.org/std/collections/index.html>
//! - <https://lib.rs/data-structures>
//! - <https://doc.rust-lang.org/std/collections/struct.HashMap.html#method.entry>
//! - <https://docs.rs/rustc-hash/latest/rustc_hash>/

use regex::Regex;
use std::sync::OnceLock;

#[expect(
    clippy::exhaustive_enums,
    reason = "cross-crate constructed; non_exhaustive => E0639"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DsaSeverity {
    P1Advisory,
    P2Warning,
}

#[expect(
    clippy::exhaustive_enums,
    reason = "cross-crate constructed; non_exhaustive => E0639"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DsaClass {
    Lookup,
    Insertion,
    Pagination,
    Sort,
    Allocation,
    Recursion,
    Hash,
}

#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate constructed; non_exhaustive => E0639"
)]
#[derive(Debug, Clone)]
pub struct DsaViolation {
    pub severity: DsaSeverity,
    pub class: DsaClass,
    pub pattern: &'static str,
    pub fix: &'static str,
    pub line: usize,
}

fn compile_regex(pat: &str) -> Regex {
    loop {
        if let Ok(re) = Regex::new(pat) {
            break re;
        }
    }
}

fn get_patterns() -> &'static Vec<Regex> {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        vec![
            compile_regex(r"(?s)\b(?:for|while)\b[^{]{0,200}\{[^}]{0,800}?\.contains\s*\(\s*&"),
            compile_regex(r"(?s)\.contains_key\s*\([^)]+\)[^;]{0,200};?[^}]{0,400}?\.insert\s*\("),
            compile_regex(r"\.insert\s*\(\s*0\s*,"),
            compile_regex(r"\.remove\s*\(\s*0\s*\)"),
            compile_regex(r"(?s)\b(?:for|while)\b[^{]{0,200}\{[^}]{0,800}?\.sort(?:_by|_unstable|_unstable_by)?\s*\("),
            compile_regex(r"(?s)\b(?:for|while)\b[^{]{0,200}\{[^}]{0,400}?\b\w+\s*\+=\s*&?[^;]*\.to_string\(\)"),
            compile_regex(r"(?s)\b(?:for|while)\b.{0,600}?format!\s*\("),
            compile_regex(r"(?s)Vec::new\s*\(\s*\).{0,800}?\b(?:for|while)\b.{0,400}?\.push\s*\("),
            compile_regex(r"(?s)HashMap::new\(\).{0,800}?(?:for|while).{0,400}?\.insert\("),
            compile_regex(r"\bLinkedList\s*<"),
            compile_regex(r"\bBTreeMap\s*<"),
            compile_regex(r"\bHashMap\s*<\s*(?:u8|u16|u32|u64|usize|i8|i16|i32|i64|isize)\b"),
            compile_regex(r"(?s)fn\s+(\w+)\s*\([^)]*\)[^{]*\{([^}]{0,2000})"),
            compile_regex(r"\.collect\s*::\s*<\s*Vec<[^>]+>\s*>\s*\(\s*\)\s*\.iter\s*\("),
            compile_regex(r"\.iter\s*\(\s*\)[^;]{0,200}\.map\s*\([^)]*\.clone\s*\(\s*\)"),
            compile_regex(r"(?s)\.sort(?:_by|_unstable)?\s*\([^)]*\)[^;]{0,200};[^}]{0,200}\.(?:iter|into_iter)\s*\(\s*\)\s*\.take\s*\("),
        ]
    })
}

fn is_backend_rust_file(path: &str, content: &str) -> bool {
    if !std::path::Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
    {
        return false;
    }
    let p = path.to_ascii_lowercase();
    if p == "build.rs" || p.ends_with("/crates/build.rs") || p == "./build.rs" {
        return false;
    }
    content.contains("async fn")
        || content.contains("axum::")
        || content.contains("tonic::")
        || content.contains("sqlx::")
        || content.contains("tokio::")
        || content.contains("Service")
        || p.contains("/handlers/")
        || p.contains("/services/")
        || p.contains("/repository/")
        || p.contains("/repo/")
        || p.contains("/domain/")
        || p.contains("/core/")
}

#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "single linear detector; splitting harms locality"
)]
pub fn detect(file_path: &str, content: &str) -> Vec<DsaViolation> {
    if !is_backend_rust_file(file_path, content) {
        return vec![];
    }
    if crate::file_types::is_test_file(file_path) {
        return vec![];
    }

    let mut v = Vec::new();
    let patterns = get_patterns();

    if patterns.first().is_some_and(|p| p.is_match(content)) {
        v.push(DsaViolation { severity: DsaSeverity::P1Advisory, class: DsaClass::Lookup,
            pattern: "vec-contains-in-loop",
            fix: "Vec::contains is O(n); calling it inside a loop = O(n^2). Build a HashSet once, then check membership in O(1).",
            line: 0 });
    }
    if patterns.get(1).is_some_and(|p| p.is_match(content)) {
        v.push(DsaViolation { severity: DsaSeverity::P1Advisory, class: DsaClass::Lookup,
            pattern: "hashmap-contains-then-insert",
            fix: "contains_key + insert = two lookups. Use map.entry(k).or_insert_with(|| ...) for a single lookup.",
            line: 0 });
    }
    if patterns.get(2).is_some_and(|p| p.is_match(content)) {
        v.push(DsaViolation { severity: DsaSeverity::P1Advisory, class: DsaClass::Insertion,
            pattern: "vec-insert-front",
            fix: "Vec::insert(0, ..) is O(n) (shifts all elements). Use VecDeque::push_front for O(1).",
            line: 0 });
    }
    if patterns.get(3).is_some_and(|p| p.is_match(content)) {
        v.push(DsaViolation {
            severity: DsaSeverity::P1Advisory,
            class: DsaClass::Insertion,
            pattern: "vec-remove-front",
            fix: "Vec::remove(0) is O(n). Use VecDeque::pop_front for O(1).",
            line: 0,
        });
    }
    if patterns.get(4).is_some_and(|p| p.is_match(content)) {
        v.push(DsaViolation { severity: DsaSeverity::P1Advisory, class: DsaClass::Sort,
            pattern: "sort-in-loop",
            fix: "Sorting inside a loop is O(n^2 log n). Sort once before the loop, or maintain a BinaryHeap / BTreeSet for incremental ordering.",
            line: 0 });
    }
    if patterns.get(5).is_some_and(|p| p.is_match(content)) {
        v.push(DsaViolation { severity: DsaSeverity::P1Advisory, class: DsaClass::Allocation,
            pattern: "string-append-in-loop",
            fix: "String += in a loop reallocates per iteration (O(n^2)). Use String::with_capacity(n) + push_str, or write! into a writer.",
            line: 0 });
    }
    if patterns.get(6).is_some_and(|p| p.is_match(content)) {
        v.push(DsaViolation { severity: DsaSeverity::P2Warning, class: DsaClass::Allocation,
            pattern: "format-in-loop",
            fix: "format!() in a loop allocates a fresh String per iteration. Use write!() into a pre-allocated buffer.",
            line: 0 });
    }
    if patterns.get(7).is_some_and(|p| p.is_match(content)) {
        v.push(DsaViolation { severity: DsaSeverity::P2Warning, class: DsaClass::Allocation,
            pattern: "vec-no-with-capacity",
            fix: "Vec::new() before a push loop reallocates as it grows. Use Vec::with_capacity(estimated_n).",
            line: 0 });
    }
    if patterns.get(8).is_some_and(|p| p.is_match(content)) {
        v.push(DsaViolation { severity: DsaSeverity::P2Warning, class: DsaClass::Allocation,
            pattern: "hashmap-no-with-capacity",
            fix: "HashMap::new() before bulk insert rehashes as it grows. Use HashMap::with_capacity(estimated_n).",
            line: 0 });
    }
    if patterns.get(9).is_some_and(|p| p.is_match(content)) {
        v.push(DsaViolation { severity: DsaSeverity::P1Advisory, class: DsaClass::Lookup,
            pattern: "linked-list-in-backend",
            fix: "LinkedList loses cache locality and is almost always slower than Vec/VecDeque for backend workloads.",
            line: 0 });
    }
    if patterns.get(10).is_some_and(|p| p.is_match(content))
        && !content.contains(".range(")
        && !content.contains("first_key_value")
    {
        v.push(DsaViolation { severity: DsaSeverity::P2Warning, class: DsaClass::Lookup,
            pattern: "btreemap-without-ordered-access",
            fix: "BTreeMap is O(log n) per lookup; only justified for range/ordered iteration. Use HashMap (O(1)) for point lookups.",
            line: 0 });
    }
    if patterns.get(11).is_some_and(|p| p.is_match(content))
        && !content.contains("FxHashMap")
        && !content.contains("rustc_hash")
    {
        v.push(DsaViolation { severity: DsaSeverity::P2Warning, class: DsaClass::Hash,
            pattern: "siphash-on-int-keys",
            fix: "HashMap with integer keys uses SipHash by default. For non-adversarial internal maps prefer FxHashMap from rustc-hash for ~2x speedup.",
            line: 0 });
    }
    let has_depth =
        content.contains("depth") || content.contains("MAX_DEPTH") || content.contains("limit");
    let recursive = patterns.get(12).is_some_and(|p| {
        p.captures_iter(content)
            .any(|cap| match (cap.get(1), cap.get(2)) {
                (Some(name), Some(body)) => {
                    let n = name.as_str();
                    if n == "main" || n.len() < 2 {
                        return false;
                    }
                    let needle = [n, "("].concat();
                    body.as_str().contains(&needle)
                }
                _ => false,
            })
    });
    if recursive && !has_depth {
        v.push(DsaViolation { severity: DsaSeverity::P1Advisory, class: DsaClass::Recursion,
            pattern: "recursion-without-depth-bound",
            fix: "Recursive fn without a depth/limit parameter risks stack overflow on adversarial input. Add depth: usize and bail when depth > MAX_DEPTH.",
            line: 0 });
    }
    if patterns.get(13).is_some_and(|p| p.is_match(content)) {
        v.push(DsaViolation { severity: DsaSeverity::P2Warning, class: DsaClass::Allocation,
            pattern: "collect-then-iter",
            fix: ".collect::<Vec<_>>().iter() allocates a Vec then re-iterates. Chain iterator adapters directly without an intermediate collect.",
            line: 0 });
    }
    if patterns.get(14).is_some_and(|p| p.is_match(content)) {
        v.push(DsaViolation { severity: DsaSeverity::P2Warning, class: DsaClass::Allocation,
            pattern: "clone-in-iter-map",
            fix: "Cloning inside .map() in an iterator chain allocates per element. Borrow with .iter() + & where possible, or .into_iter() if you can consume.",
            line: 0 });
    }
    if patterns.get(15).is_some_and(|p| p.is_match(content)) {
        v.push(DsaViolation { severity: DsaSeverity::P1Advisory, class: DsaClass::Sort,
            pattern: "full-sort-then-take",
            fix: "Sorting full Vec then taking n elements = O(n log n) when O(n) is possible. Use Vec::select_nth_unstable or BinaryHeap for top-k.",
            line: 0 });
    }

    v
}

#[must_use]
pub fn warn_count(file_path: &str, content: &str) -> usize {
    detect(file_path, content).len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec_contains_in_loop_flagged() {
        let src = r"
use sqlx;
async fn x() {}
fn check(items: Vec<u64>, ids: Vec<u64>) -> usize {
    let mut c = 0;
    for id in ids { if items.contains(&id) { c += 1; } }
    c
}
";
        let r = detect("src/services/check.rs", src);
        assert!(r.iter().any(|v| v.pattern == "vec-contains-in-loop"));
    }

    #[test]
    fn hashmap_contains_then_insert_flagged() {
        let src = r"
use sqlx;
use std::collections::HashMap;
async fn x() {}
fn add(m: &mut HashMap<String, u64>, k: String) {
    if !m.contains_key(&k) { m.insert(k, 0); }
}
";
        let r = detect("src/services/cache.rs", src);
        assert!(
            r.iter()
                .any(|v| v.pattern == "hashmap-contains-then-insert")
        );
    }

    #[test]
    fn vec_insert_front_flagged() {
        let src = r"
use sqlx;
async fn x() {}
fn front(v: &mut Vec<u64>) { v.insert(0, 42); }
";
        let r = detect("src/services/q.rs", src);
        assert!(r.iter().any(|v| v.pattern == "vec-insert-front"));
    }

    #[test]
    fn vec_remove_front_flagged() {
        let src = r"
use sqlx;
async fn x() {}
fn deq(v: &mut Vec<u64>) -> u64 { v.remove(0) }
";
        let r = detect("src/services/q.rs", src);
        assert!(r.iter().any(|v| v.pattern == "vec-remove-front"));
    }

    #[test]
    fn sort_in_loop_flagged() {
        let src = r"
use sqlx;
async fn x() {}
fn ranks(items: &mut Vec<Vec<u64>>) {
    for inner in items { inner.sort(); }
}
";
        let r = detect("src/services/rank.rs", src);
        assert!(r.iter().any(|v| v.pattern == "sort-in-loop"));
    }

    #[test]
    fn string_append_in_loop_flagged() {
        let src = r"
use sqlx;
async fn x() {}
fn build(parts: Vec<&str>) -> String {
    let mut s = String::new();
    for p in parts { s += &p.to_string(); }
    s
}
";
        let r = detect("src/services/build.rs", src);
        assert!(r.iter().any(|v| v.pattern == "string-append-in-loop"));
    }

    #[test]
    fn format_in_loop_flagged() {
        let src = r#"
use sqlx;
async fn x() {}
fn build(ids: Vec<u64>) -> Vec<String> {
    let mut out = Vec::new();
    for id in ids { out.push(format!("k:{}", id)); }
    out
}
"#;
        let r = detect("src/services/build.rs", src);
        assert!(r.iter().any(|v| v.pattern == "format-in-loop"));
    }

    #[test]
    fn vec_no_capacity_flagged() {
        let src = r"
use sqlx;
async fn x() {}
fn build(ids: Vec<u64>) -> Vec<u64> {
    let mut out = Vec::new();
    for id in ids { out.push(id); }
    out
}
";
        let r = detect("src/services/build.rs", src);
        assert!(r.iter().any(|v| v.pattern == "vec-no-with-capacity"));
    }

    #[test]
    fn hashmap_no_capacity_flagged() {
        let src = r"
use sqlx;
use std::collections::HashMap;
async fn x() {}
fn build(ids: Vec<u64>) -> HashMap<u64, u64> {
    let mut out = HashMap::new();
    for id in ids { out.insert(id, 0); }
    out
}
";
        let r = detect("src/services/build.rs", src);
        assert!(r.iter().any(|v| v.pattern == "hashmap-no-with-capacity"));
    }

    #[test]
    fn linked_list_flagged() {
        let src = r"
use sqlx;
use std::collections::LinkedList;
async fn x() {}
fn make() -> LinkedList<u64> { LinkedList::new() }
";
        let r = detect("src/services/list.rs", src);
        assert!(r.iter().any(|v| v.pattern == "linked-list-in-backend"));
    }

    #[test]
    fn btreemap_without_range_flagged() {
        let src = r"
use sqlx;
use std::collections::BTreeMap;
async fn x() {}
fn make() -> BTreeMap<u64, u64> { BTreeMap::new() }
";
        let r = detect("src/services/map.rs", src);
        assert!(
            r.iter()
                .any(|v| v.pattern == "btreemap-without-ordered-access")
        );
    }

    #[test]
    fn btreemap_with_range_ok() {
        let src = r"
use sqlx;
use std::collections::BTreeMap;
async fn x() {}
fn lookup(m: &BTreeMap<u64, u64>) { let _ = m.range(0..100); }
";
        let r = detect("src/services/map.rs", src);
        assert!(
            !r.iter()
                .any(|v| v.pattern == "btreemap-without-ordered-access")
        );
    }

    #[test]
    fn siphash_on_int_keys_flagged() {
        let src = r"
use sqlx;
use std::collections::HashMap;
async fn x() {}
fn make() -> HashMap<u64, u64> { HashMap::new() }
";
        let r = detect("src/services/idmap.rs", src);
        assert!(r.iter().any(|v| v.pattern == "siphash-on-int-keys"));
    }

    #[test]
    fn fxhashmap_ok() {
        let src = r"
use sqlx;
use rustc_hash::FxHashMap;
async fn x() {}
fn make() -> FxHashMap<u64, u64> { FxHashMap::default() }
";
        let r = detect("src/services/idmap.rs", src);
        assert!(!r.iter().any(|v| v.pattern == "siphash-on-int-keys"));
    }

    #[test]
    fn recursion_without_depth_flagged() {
        let src = r"
use sqlx;
async fn x() {}
fn walk(node: u64) -> u64 { walk(node + 1) }
";
        let r = detect("src/services/walk.rs", src);
        assert!(
            r.iter()
                .any(|v| v.pattern == "recursion-without-depth-bound")
        );
    }

    #[test]
    fn recursion_with_depth_ok() {
        let src = r"
use sqlx;
const MAX_DEPTH: usize = 100;
async fn x() {}
fn walk(node: u64, depth: usize) -> u64 { if depth > MAX_DEPTH { return 0; } walk(node + 1, depth + 1) }
";
        let r = detect("src/services/walk.rs", src);
        assert!(
            !r.iter()
                .any(|v| v.pattern == "recursion-without-depth-bound")
        );
    }

    #[test]
    fn collect_then_iter_flagged() {
        let src = r"
use sqlx;
async fn x() {}
fn squares(v: Vec<u64>) -> u64 {
    v.iter().map(|x| x * x).collect::<Vec<u64>>().iter().sum()
}
";
        let r = detect("src/services/sq.rs", src);
        assert!(r.iter().any(|v| v.pattern == "collect-then-iter"));
    }

    #[test]
    fn clone_in_iter_map_flagged() {
        let src = r"
use sqlx;
async fn x() {}
fn names(items: &Vec<String>) -> Vec<String> {
    items.iter().map(|s| s.clone()).collect()
}
";
        let r = detect("src/services/names.rs", src);
        assert!(r.iter().any(|v| v.pattern == "clone-in-iter-map"));
    }

    #[test]
    fn full_sort_then_take_flagged() {
        let src = r"
use sqlx;
async fn x() {}
fn top3(mut v: Vec<u64>) -> Vec<u64> {
    v.sort();
    v.iter().take(3).copied().collect()
}
";
        let r = detect("src/services/topk.rs", src);
        assert!(r.iter().any(|v| v.pattern == "full-sort-then-take"));
    }

    #[test]
    fn non_rust_skipped() {
        let r = detect(
            "src/index.ts",
            "for (const x of xs) { if (xs.contains(x)) {} }",
        );
        assert!(r.is_empty());
    }

    #[test]
    fn test_file_skipped() {
        let src = r"
use sqlx;
async fn x() {}
fn check(items: Vec<u64>) {
    for id in items { let _ = items.contains(&id); }
}
";
        let r = detect("crate/tests/dsa.rs", src);
        assert!(r.is_empty());
    }

    #[test]
    fn warn_count_works() {
        let src = r"
use sqlx;
async fn x() {}
fn build(ids: Vec<u64>) -> Vec<u64> {
    let mut out = Vec::new();
    for id in ids { out.push(id); }
    out
}
";
        assert!(warn_count("src/services/x.rs", src) >= 1);
    }
}
