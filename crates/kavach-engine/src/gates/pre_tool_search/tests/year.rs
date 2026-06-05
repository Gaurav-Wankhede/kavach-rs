//! Stale-year detection tests + the empty-query `run` smoke test.
use kavach_types::HookInput;

use crate::gates::pre_tool_search::run;
use crate::gates::pre_tool_search::year::check_stale_year_in_query;

#[test]
fn should_block_stale_year_2025_when_current_2026() {
    assert!(check_stale_year_in_query("Astro 5 dashboard 2025", 2026).is_some());
}

#[test]
fn should_allow_current_year_query() {
    assert!(check_stale_year_in_query("Astro 6 dashboard 2026", 2026).is_none());
}

#[test]
fn should_allow_future_year() {
    assert!(check_stale_year_in_query("roadmap 2027", 2026).is_none());
}

#[test]
fn should_allow_query_without_year() {
    assert!(check_stale_year_in_query("Astro middleware patterns", 2026).is_none());
}

#[test]
fn should_not_flag_years_before_2020() {
    assert!(check_stale_year_in_query("RFC 1918 private addresses", 2026).is_none());
}

#[test]
fn should_block_stale_2024_when_current_2026() {
    assert!(check_stale_year_in_query("React 18 patterns 2024", 2026).is_some());
}

#[test]
fn test_run_empty_query() {
    run(&HookInput::default());
}
