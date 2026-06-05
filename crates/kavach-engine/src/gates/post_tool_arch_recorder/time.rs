//! Time utilities for arch recorder.

/// Seconds per year (approximate).
const SECS_PER_YEAR: u64 = 31_557_600;
/// Seconds per day.
const SECS_PER_DAY: u64 = 86_400;
/// Days per year.
const DAYS_PER_YEAR: u64 = 365;
/// Days per month (approximate).
const DAYS_PER_MONTH: u64 = 31;
/// Unix epoch year.
const EPOCH_YEAR: i64 = 1970;
/// Fallback year.
const FALLBACK_YEAR: i64 = 2026;

pub(crate) fn current_year() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| {
            #[expect(clippy::integer_division, reason = "SECS_PER_YEAR is const non-zero")]
            let y = d.as_secs() / SECS_PER_YEAR;
            i64::try_from(y).ok()
        })
        .map_or(FALLBACK_YEAR, |y| {
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "adding year offset to bounded EPOCH_YEAR"
            )]
            let year = EPOCH_YEAR + y;
            year
        })
}

pub(crate) fn current_month() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| {
            #[expect(clippy::integer_division, reason = "SECS_PER_DAY is const non-zero")]
            let secs_per_day_div = d.as_secs() / SECS_PER_DAY;
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "modulo + 1 on bounded day values within [1..366]"
            )]
            let day_of_year = secs_per_day_div % DAYS_PER_YEAR + 1;
            #[expect(clippy::integer_division, reason = "DAYS_PER_MONTH is const non-zero")]
            let month_div = day_of_year / DAYS_PER_MONTH;
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "adding 1 to bounded month value clamped to [0..11]"
            )]
            let month = month_div.min(11) + 1;
            month
        })
        .and_then(|m| i64::try_from(m).ok())
        .unwrap_or(1)
}
