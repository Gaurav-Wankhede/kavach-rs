//! Stale-year detection: a `WebSearch` query naming an older year than current
//! is steering toward stale (training-weight) versions.

/// Returns block reason if query contains a year from `2020..current_year` (exclusive).
pub(in crate::gates::pre_tool_search) fn check_stale_year_in_query(
    query: &str,
    current_year: u32,
) -> Option<String> {
    for word in query.split_whitespace() {
        let digits: String = word.chars().filter(char::is_ascii_digit).collect();
        if digits.len() == 4
            && let Ok(year) = digits.parse::<u32>()
            && year >= 2020
            && year < current_year
        {
            return Some(format!(
                "STALE_YEAR_BLOCKED: Query contains year {year} but current year is {current_year}.\n\
                         Training weights are stale — do NOT search for old versions.\n\
                         FIX: Replace {year} with {current_year} in your search query."
            ));
        }
    }
    None
}
