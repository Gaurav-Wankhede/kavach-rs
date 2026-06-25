use super::dispatch::{detect, warn_count};

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
