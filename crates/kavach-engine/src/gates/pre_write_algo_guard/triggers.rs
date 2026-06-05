//! Curated algorithmic / data-structure trigger keywords.
//!
//! ARCH: `AlgoTriggerKeywords` — curated to minimize false positives.
//! PATTERN: `keyword_filter` | SCOPE: `pre_write` | CAP: AP | SEARCHED: 2026-04
//! Per SAST research: simple keyword matching causes 50%+ false positives.
//! Only data-structure type names, not generic terms like "cache".

/// Keywords that indicate algorithmic / data-structure decisions. Matched via
/// `contains` linear scan (file content is small).
pub(super) const ALGO_TRIGGERS: &[&str] = &[
    "BTreeMap",
    "BTreeSet",
    "BinaryHeap",
    "HashMap",
    "HashSet",
    "IndexMap",
    "LruCache",
    "SkipList",
    "VecDeque",
    "bloom",
    // "cache" removed — false positives on Cache-Control headers, cache-busting
    // URLs. `LruCache` triggers via "lru". Explicit cache crates (moka,
    // quick_cache) use their names.
    // "dedup" removed — Vec::dedup() is stdlib, not an algorithmic decision.
    "hash_map",
    "hash_set",
    "heap",
    "index_map",
    "lru",
    "merge_sort",
    // "partition" removed — Iterator/slice::partition() are stdlib, not choices.
    "quicksort",
    "radix",
    "segment_tree",
    "skip_list",
    // "sort_by" removed — Vec::sort_by() is stdlib; fires on any comparison sort.
    "sort_unstable",
    "trie",
    "union_find",
];
