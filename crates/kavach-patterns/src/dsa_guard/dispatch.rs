use super::patterns::{get_patterns, is_backend_rust_file};
use super::types::{DsaClass, DsaSeverity, DsaViolation};

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
            fix: "Cloning inside .map() in an iterator chain allocates per element. Borrow with .iter() + &, or .into_iter() to consume.",
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
