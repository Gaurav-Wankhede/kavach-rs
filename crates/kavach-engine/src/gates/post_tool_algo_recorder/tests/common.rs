//! Shared test fixture: a full `// ALGO:` comment block with no BENCHMARK field
//! (so unit tests make no network calls).

/// SEARCHED is on its own line so `extract_field` can find it.
pub(super) fn full_comment(search_year: i64, search_month: i64, year_published: i64) -> String {
    format!(
        "// ALGO: pdqsort\n\
         // TIME: O(n log n) | SPACE: O(log n)\n\
         // YEAR: {year_published}\n\
         // SEARCHED: {search_year}-{search_month:02}\n\
         fn sort_items() {{}}"
    )
}
