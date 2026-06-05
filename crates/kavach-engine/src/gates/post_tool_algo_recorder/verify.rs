//! Verify the structured `// ALGO:` fields before recording: SEARCHED freshness,
//! plausible publication year, and a reachable BENCHMARK URL.
use super::datetime::current_year;
use super::parse::AlgoComment;

/// Verify the comment fields. `Ok(())` on pass, `Err(reason)` on failure.
///
/// Checks:
/// 1. `SEARCHED` year ∈ [current_year-1, `current_year`] — not stale, not future.
/// 2. `YEAR` publication year is plausible (1950 ≤ year ≤ `current_year`).
/// 3. `BENCHMARK` URL returns a 2xx/3xx HTTP status.
pub(super) fn verify_algo_comment(algo: &AlgoComment) -> Result<(), String> {
    let now_year = current_year();
    if algo.search_year < now_year.saturating_sub(1) {
        return Err(format!(
            "SEARCHED year {} is stale (current: {now_year}). Re-run /arch.",
            algo.search_year
        ));
    }
    if algo.search_year > now_year {
        return Err(format!(
            "SEARCHED year {} is in the future (current: {now_year}). Check SEARCHED field.",
            algo.search_year
        ));
    }
    if algo.year_published != 0 && (algo.year_published < 1950 || algo.year_published > now_year) {
        return Err(format!(
            "YEAR {} is implausible (valid range: 1950–{now_year}). Verify publication year.",
            algo.year_published
        ));
    }
    if let Some(ref url) = algo.benchmark_source
        && !url.is_empty()
    {
        match verify_url(url) {
            Ok(true) => {}
            Ok(false) => {
                return Err(format!(
                    "BENCHMARK URL returned 404 or unreachable: {url}. \
                     Provide a valid crates.io or arXiv URL."
                ));
            }
            Err(e) => {
                return Err(format!(
                    "BENCHMARK URL could not be verified ({e}): {url}. \
                     Check network or provide a reachable URL."
                ));
            }
        }
    }
    Ok(())
}

/// Check URL reachability via toolbelt (xh with curl fallback).
/// `Ok(true)` for 2xx/3xx, `Ok(false)` for 4xx/5xx, `Err` if the request fails.
fn verify_url(url: &str) -> Result<bool, String> {
    crate::toolbelt::verify_url_reachable(url, 5)
}
